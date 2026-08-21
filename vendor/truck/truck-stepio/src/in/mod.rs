#![allow(missing_docs, unused_qualifications)]

/// re-export [`ruststep`](https://docs.rs/ruststep/latest/ruststep/)
pub use ruststep;

use ruststep::{
    ast::{DataSection, EntityInstance, Name, Parameter, Record, SubSuperRecord},
    primitive::Logical,
    tables::{EntityTable, IntoOwned, PlaceHolder},
    Holder,
};
use serde::{Deserialize, Serialize};
use std::result::Result;
use std::{collections::HashMap, f64::consts::PI};
use truck_assembly::assy::*;
use truck_geometry::prelude as truck;
use truck_topology::compress::*;

/// Typed entity identities and the transactional arenas that convert into them
pub mod arena;
use arena::*;

/// Face boundaries that are known to close
pub mod wire;
use wire::*;

pub mod convert;
/// Geometry parsed from STEP that can be handled by truck
pub mod step_geometry;
use step_geometry::*;

/// Typed presentation entities (ISO 10303-46 subset the corpora use).
pub mod presentation;

/// the exchange structure corresponds to a graph in STEP file
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Table {
    // representation
    pub representation: HashMap<u64, RepresentationHolder>,
    pub representation_item: HashMap<u64, RepresentationItemHolder>,
    pub representation_context: HashMap<u64, RepresentationContextHolder>,

    // primitives
    pub cartesian_point: HashMap<u64, CartesianPointHolder>,
    pub direction: HashMap<u64, DirectionHolder>,
    pub vector: HashMap<u64, VectorHolder>,
    pub placement: HashMap<u64, PlacementHolder>,
    pub axis1_placement: HashMap<u64, Axis1PlacementHolder>,
    pub axis2_placement_2d: HashMap<u64, Axis2Placement2dHolder>,
    pub axis2_placement_3d: HashMap<u64, Axis2Placement3dHolder>,

    // curve
    pub line: HashMap<u64, LineHolder>,
    pub polyline: HashMap<u64, PolylineHolder>,
    pub b_spline_curve_with_knots: HashMap<u64, BSplineCurveWithKnotsHolder>,
    pub bezier_curve: HashMap<u64, BezierCurveHolder>,
    pub quasi_uniform_curve: HashMap<u64, QuasiUniformCurveHolder>,
    pub uniform_curve: HashMap<u64, UniformCurveHolder>,
    pub rational_b_spline_curve: HashMap<u64, RationalBSplineCurveHolder>,
    pub circle: HashMap<u64, CircleHolder>,
    pub ellipse: HashMap<u64, EllipseHolder>,
    pub hyperbola: HashMap<u64, HyperbolaHolder>,
    pub parabola: HashMap<u64, ParabolaHolder>,
    pub pcurve: HashMap<u64, PcurveHolder>,
    pub surface_curve: HashMap<u64, SurfaceCurveHolder>,

    // surface
    pub plane: HashMap<u64, PlaneHolder>,
    pub spherical_surface: HashMap<u64, SphericalSurfaceHolder>,
    pub offset_surface: HashMap<u64, OffsetSurfaceHolder>,
    pub cylindrical_surface: HashMap<u64, CylindricalSurfaceHolder>,
    pub toroidal_surface: HashMap<u64, ToroidalSurfaceHolder>,
    pub degenerate_toroidal_surface: HashMap<u64, DegenerateToroidalSurfaceHolder>,
    pub conical_surface: HashMap<u64, ConicalSurfaceHolder>,
    pub b_spline_surface_with_knots: HashMap<u64, BSplineSurfaceWithKnotsHolder>,
    pub uniform_surface: HashMap<u64, UniformSurfaceHolder>,
    pub quasi_uniform_surface: HashMap<u64, QuasiUniformSurfaceHolder>,
    pub bezier_surface: HashMap<u64, BezierSurfaceHolder>,
    pub rational_b_spline_surface: HashMap<u64, RationalBSplineSurfaceHolder>,
    pub surface_of_linear_extrusion: HashMap<u64, SurfaceOfLinearExtrusionHolder>,
    pub surface_of_revolution: HashMap<u64, SurfaceOfRevolutionHolder>,

    // topology
    pub vertex_point: HashMap<u64, VertexPointHolder>,
    pub edge_curve: HashMap<u64, EdgeCurveHolder>,
    pub oriented_edge: HashMap<u64, OrientedEdgeHolder>,
    pub edge_loop: HashMap<u64, EdgeLoopHolder>,
    pub vertex_loop: HashMap<u64, VertexLoopHolder>,
    pub face_bound: HashMap<u64, FaceBoundHolder>,
    /// The ids in `face_bound` that arrived as `FACE_OUTER_BOUND` rather than
    /// as a plain `FACE_BOUND`.
    ///
    /// Both entity types deserialize into `FaceBoundHolder`, which has no field
    /// for the distinction, so without this set the standing is erased at parse
    /// and no later stage can recover it. STEP does distinguish them, and the
    /// material region of a face with an inner loop is not the material region
    /// of the same loops read as outer ones.
    pub face_outer_bound_ids: std::collections::HashSet<u64>,
    pub face_surface: HashMap<u64, FaceSurfaceHolder>,
    pub oriented_face: HashMap<u64, OrientedFaceHolder>,
    pub shell: HashMap<u64, ShellHolder>,
    pub oriented_shell: HashMap<u64, OrientedShellHolder>,
    pub shell_based_surface_model: HashMap<u64, ShellBasedSurfaceModelHolder>,
    pub manifold_solid_brep: HashMap<u64, ManifoldSolidBrepHolder>,

    // assembly
    pub application_context: HashMap<u64, ApplicationContextHolder>,
    pub product_context: HashMap<u64, ProductContextHolder>,
    pub product: HashMap<u64, ProductHolder>,
    pub product_definition_formation: HashMap<u64, ProductDefinitionFormationHolder>,
    pub product_definition_context: HashMap<u64, ProductDefinitionContextHolder>,
    pub product_definition: HashMap<u64, ProductDefinitionHolder>,
    pub product_definition_shape: HashMap<u64, ProductDefinitionShapeHolder>,
    pub shape_definition_representation: HashMap<u64, ShapeDefinitionRepresentationHolder>,
    pub shape_representation: HashMap<u64, ShapeRepresentationHolder>,
    pub context_dependent_shape_representation:
        HashMap<u64, ContextDependentShapeRepresentationHolder>,
    pub shape_representation_relationship: HashMap<u64, ShapeRepresentationRelationshipHolder>,
    pub shape_representation_relationship_with_transformation:
        HashMap<u64, ShapeRepresentationRelationshipWithTransformationHolder>,
    pub next_assembly_usage_occurrence: HashMap<u64, NextAssemblyUsageOccurrenceHolder>,
    pub item_defined_transformation: HashMap<u64, ItemDefinedTransformationHolder>,

    // presentation (ISO 10303-46)
    //
    // The typed surface/face colour chains. Each holder preserves the source
    // entity's references as raw ids; nothing here interprets what they mean.
    pub colour_rgb: HashMap<u64, presentation::ColourRgbHolder>,
    pub draughting_pre_defined_colour: HashMap<u64, presentation::DraughtingPreDefinedColourHolder>,
    pub styled_item: HashMap<u64, presentation::StyledItemHolder>,
    pub over_riding_styled_item: HashMap<u64, presentation::OverRidingStyledItemHolder>,
    pub presentation_style_assignment:
        HashMap<u64, presentation::PresentationStyleAssignmentHolder>,
    pub surface_style_usage: HashMap<u64, presentation::SurfaceStyleUsageHolder>,
    pub surface_side_style: HashMap<u64, presentation::SurfaceSideStyleHolder>,
    pub surface_style_fill_area: HashMap<u64, presentation::SurfaceStyleFillAreaHolder>,
    pub fill_area_style: HashMap<u64, presentation::FillAreaStyleHolder>,
    pub fill_area_style_colour: HashMap<u64, presentation::FillAreaStyleColourHolder>,

    // others
    pub definitional_representation: HashMap<u64, DefinitionalRepresentationHolder>,

    // dummy
    pub dummy: HashMap<u64, DummyHolder>,

    /// Every `PLANE_ANGLE_UNIT` the file declares, in the order found.
    ///
    /// Angles are the one imported quantity whose unit cannot be ignored. A
    /// length unit is a uniform scale, so a file in inches renders identically
    /// to the same file in millimetres — the tolerance is relative and nothing
    /// downstream cares. An angle is not scale-covariant: mixing an angle in
    /// degrees with lengths in any unit is dimensionally inconsistent, and the
    /// error is not a scale factor but a different shape.
    ///
    /// That is why the omission stayed invisible and then produced a blob. NIST
    /// `ftc_07` declares degrees and writes a 2° draft cone, which was read as
    /// 2 radians: `tan(2°) = 0.035` against `tan(2 rad) = −2.185`, so the cone
    /// flared backwards at 63x the intended slope and four corner fillets
    /// became fans bursting out of the part.
    pub plane_angle_units: Vec<(u64, PlaneAngleUnit)>,
    /// `PLANE_ANGLE_MEASURE_WITH_UNIT` by entity id: radians per unit, and the
    /// unit that measurement is expressed *in*.
    ///
    /// The second field is what stops the base unit of a conversion from being
    /// mistaken for a competing declaration. A degree unit is defined as
    /// "0.0174532925 of that radian unit over there", so every file using
    /// degrees necessarily also contains a radian `SI_UNIT` — referenced, not
    /// assigned. Ignoring that cost one wrong refusal before it was noticed.
    pub plane_angle_measures: HashMap<u64, (f64, Option<u64>)>,
    /// `UNCERTAINTY_MEASURE_WITH_UNIT` by entity id, in the file's native
    /// length units.
    ///
    /// The declared geometric-uncertainty of a shape's representation context:
    /// "the maximum model space distance between geometric entities at asserted
    /// connectivities". It is the tolerance under which the file asserts that a
    /// source vertex lies on an edge-curve carrier. It is read in native units
    /// because model coordinates are left in native units; converting the value
    /// to millimetres while the geometry stays in inches would be wrong by the
    /// same scale factor the rest of the importer deliberately ignores.
    pub uncertainty_measures: HashMap<u64, f64>,
    /// `GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT` complex entities, by context
    /// entity id.
    ///
    /// A geometric representation context that assigns an uncertainty is the
    /// complex entity
    ///
    /// ```text
    /// #c = ( GEOMETRIC_REPRESENTATION_CONTEXT(3)
    ///        GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#u))
    ///        ... REPRESENTATION_CONTEXT('ctx', '3D') );
    /// ```
    ///
    /// so the uncertainty belongs to the context entity, not to the
    /// `REPRESENTATION_CONTEXT` record inside it. This map carries the
    /// assignment so a shape representation can be resolved to its uncertainty
    /// by way of its `context_of_items` reference.
    pub global_uncertainty_assigned_contexts: HashMap<u64, Vec<u64>>,
}

/// A `PLANE_ANGLE_UNIT` declaration, reduced to what conversion needs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlaneAngleUnit {
    /// `SI_UNIT($, .RADIAN.)` — already the internal unit, factor 1.
    Radian,
    /// `CONVERSION_BASED_UNIT`, whose factor is the referenced measure.
    Converted {
        /// The `PLANE_ANGLE_MEASURE_WITH_UNIT` holding radians per unit.
        measure: u64,
    },
}

/// The semantic dimension of a STEP parameter value, determined by its consumer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterDimension {
    /// Angular parameter on a periodic/rotational curve or surface (Circle, Ellipse, ConicalSurface).
    PlaneAngle,
    /// Spatial length parameter (Line parameter, offset distance).
    Length,
    /// Dimensionless parameter (BSpline/NURBS knot vectors).
    Dimensionless,
    /// Unconverted native parameter.
    NativeCurveParameter,
}

impl ParameterDimension {
    /// Determine parameter dimension based on the consuming basis curve type.
    pub fn for_basis_curve(curve: &CurveAny) -> Self {
        match curve {
            CurveAny::Conic(conic) => match conic.as_ref() {
                Conic::Circle(_) | Conic::Ellipse(_) => Self::PlaneAngle,
                Conic::Hyperbola(_) | Conic::Parabola(_) => Self::NativeCurveParameter,
            },
            CurveAny::Line(_) => Self::NativeCurveParameter,
            CurveAny::BoundedCurve(_) => Self::Dimensionless,
            CurveAny::Pcurve(_) | CurveAny::SurfaceCurve(_) => Self::NativeCurveParameter,
        }
    }
}

/// Convert a raw PARAMETER_VALUE according to its resolved parameter dimension and table unit context.
pub fn convert_parameter_value(
    value: f64,
    dimension: ParameterDimension,
    plane_angle_factor: f64,
) -> f64 {
    match dimension {
        ParameterDimension::PlaneAngle => value * plane_angle_factor,
        ParameterDimension::Length
        | ParameterDimension::Dimensionless
        | ParameterDimension::NativeCurveParameter => value,
    }
}

impl Table {
    /// Parse the presentation entities this reader knows, into their typed
    /// tables.
    ///
    /// Returns `true` when `name` is one of the presentation entities and the
    /// record has been consumed (into a typed holder, or into `dummy` when it
    /// could not be parsed — the same destination unknown records reach, so a
    /// file with a presentation shape this reader does not understand still
    /// loads). Returns `false` for any other name so the caller falls through
    /// to the regular geometry/topology dispatch.
    pub fn push_presentation(&mut self, id: u64, record: &Record) -> bool {
        let fallback = |this: &mut Self| {
            this.dummy.insert(
                id,
                DummyHolder {
                    record: format!("{record:?}"),
                    is_simple: true,
                },
            );
        };
        match record.name.as_str() {
            "COLOUR_RGB" => match presentation::colour_rgb(&record.parameter) {
                Some(holder) => {
                    self.colour_rgb.insert(id, holder);
                }
                None => fallback(self),
            },
            "DRAUGHTING_PRE_DEFINED_COLOUR" => {
                match presentation::draughting_pre_defined_colour(&record.parameter) {
                    Some(holder) => {
                        self.draughting_pre_defined_colour.insert(id, holder);
                    }
                    None => fallback(self),
                }
            }
            "STYLED_ITEM" => match presentation::styled_item(&record.parameter) {
                Some(holder) => {
                    self.styled_item.insert(id, holder);
                }
                None => fallback(self),
            },
            "OVER_RIDING_STYLED_ITEM" => {
                match presentation::over_riding_styled_item(&record.parameter) {
                    Some(holder) => {
                        self.over_riding_styled_item.insert(id, holder);
                    }
                    None => fallback(self),
                }
            }
            "PRESENTATION_STYLE_ASSIGNMENT" => {
                match presentation::presentation_style_assignment(&record.parameter) {
                    Some(holder) => {
                        self.presentation_style_assignment.insert(id, holder);
                    }
                    None => fallback(self),
                }
            }
            "SURFACE_STYLE_USAGE" => match presentation::surface_style_usage(&record.parameter) {
                Some(holder) => {
                    self.surface_style_usage.insert(id, holder);
                }
                None => fallback(self),
            },
            "SURFACE_SIDE_STYLE" => match presentation::surface_side_style(&record.parameter) {
                Some(holder) => {
                    self.surface_side_style.insert(id, holder);
                }
                None => fallback(self),
            },
            "SURFACE_STYLE_FILL_AREA" => {
                match presentation::surface_style_fill_area(&record.parameter) {
                    Some(holder) => {
                        self.surface_style_fill_area.insert(id, holder);
                    }
                    None => fallback(self),
                }
            }
            "FILL_AREA_STYLE" => match presentation::fill_area_style(&record.parameter) {
                Some(holder) => {
                    self.fill_area_style.insert(id, holder);
                }
                None => fallback(self),
            },
            "FILL_AREA_STYLE_COLOUR" => {
                match presentation::fill_area_style_colour(&record.parameter) {
                    Some(holder) => {
                        self.fill_area_style_colour.insert(id, holder);
                    }
                    None => fallback(self),
                }
            }
            _ => return false,
        };
        true
    }

    pub fn push_instance(&mut self, instance: &EntityInstance) -> ruststep::error::Result<()> {
        match instance {
            EntityInstance::Simple { id, record } => {
                if self.push_presentation(*id, record) {
                    return Ok(());
                }
                match record.name.as_str() {
                    "CARTESIAN_POINT" => {
                        self.cartesian_point
                            .insert(*id, Deserialize::deserialize(record)?);
                    }
                    "DIRECTION" => {
                        self.direction
                            .insert(*id, Deserialize::deserialize(record)?);
                    }
                    "VECTOR" => {
                        self.vector.insert(*id, Deserialize::deserialize(record)?);
                    }
                    "PLACEMENT" => {
                        self.placement
                            .insert(*id, Deserialize::deserialize(record)?);
                    }
                    "AXIS1_PLACEMENT" => {
                        self.axis1_placement
                            .insert(*id, Deserialize::deserialize(&record.parameter)?);
                    }
                    "AXIS2_PLACEMENT_2D" => {
                        self.axis2_placement_2d
                            .insert(*id, Deserialize::deserialize(&record.parameter)?);
                    }
                    "AXIS2_PLACEMENT_3D" => {
                        self.axis2_placement_3d
                            .insert(*id, Deserialize::deserialize(&record.parameter)?);
                    }
                    "LINE" => {
                        self.line
                            .insert(*id, Deserialize::deserialize(&record.parameter)?);
                    }
                    "POLYLINE" => {
                        self.polyline.insert(*id, Deserialize::deserialize(record)?);
                    }
                    "B_SPLINE_CURVE_WITH_KNOTS" => {
                        self.b_spline_curve_with_knots
                            .insert(*id, Deserialize::deserialize(record)?);
                    }
                    "BEZIER_CURVE" => {
                        self.bezier_curve
                            .insert(*id, Deserialize::deserialize(record)?);
                    }
                    "QUASI_UNIFORM_CURVE" => {
                        self.quasi_uniform_curve
                            .insert(*id, Deserialize::deserialize(record)?);
                    }
                    "UNIFORM_CURVE" => {
                        self.uniform_curve
                            .insert(*id, Deserialize::deserialize(record)?);
                    }
                    "CIRCLE" => {
                        self.circle.insert(*id, Deserialize::deserialize(record)?);
                    }
                    "ELLIPSE" => {
                        self.ellipse.insert(*id, Deserialize::deserialize(record)?);
                    }
                    "HYPERBOLA" => {
                        self.hyperbola
                            .insert(*id, Deserialize::deserialize(record)?);
                    }
                    "PARABOLA" => {
                        self.parabola.insert(*id, Deserialize::deserialize(record)?);
                    }
                    "PCURVE" => {
                        self.pcurve.insert(*id, Deserialize::deserialize(record)?);
                    }
                    "SURFACE_CURVE" => {
                        self.surface_curve
                            .insert(*id, Deserialize::deserialize(record)?);
                    }
                    "SEAM_CURVE" => {
                        self.surface_curve
                            .insert(*id, Deserialize::deserialize(&record.parameter)?);
                    }
                    "PLANE" => {
                        self.plane.insert(*id, Deserialize::deserialize(record)?);
                    }
                    "OFFSET_SURFACE" => {
                        self.offset_surface
                            .insert(*id, Deserialize::deserialize(record)?);
                    }
                    "SPHERICAL_SURFACE" => {
                        self.spherical_surface
                            .insert(*id, Deserialize::deserialize(record)?);
                    }
                    "CYLINDRICAL_SURFACE" => {
                        self.cylindrical_surface
                            .insert(*id, Deserialize::deserialize(record)?);
                    }
                    "TOROIDAL_SURFACE" => {
                        self.toroidal_surface
                            .insert(*id, Deserialize::deserialize(record)?);
                    }
                    "DEGENERATE_TOROIDAL_SURFACE" => {
                        self.degenerate_toroidal_surface
                            .insert(*id, Deserialize::deserialize(record)?);
                    }
                    "CONICAL_SURFACE" => {
                        self.conical_surface
                            .insert(*id, Deserialize::deserialize(record)?);
                    }
                    "B_SPLINE_SURFACE_WITH_KNOTS" => {
                        self.b_spline_surface_with_knots
                            .insert(*id, Deserialize::deserialize(record)?);
                    }
                    "UNIFORM_SURFACE" => {
                        self.uniform_surface
                            .insert(*id, Deserialize::deserialize(record)?);
                    }
                    "QUASI_UNIFORM_SURFACE" => {
                        self.quasi_uniform_surface
                            .insert(*id, Deserialize::deserialize(record)?);
                    }
                    "BEZIER_SURFACE" => {
                        self.bezier_surface
                            .insert(*id, Deserialize::deserialize(record)?);
                    }
                    "SURFACE_OF_LINEAR_EXTRUSION" => {
                        self.surface_of_linear_extrusion
                            .insert(*id, Deserialize::deserialize(record)?);
                    }
                    "SURFACE_OF_REVOLUTION" => {
                        self.surface_of_revolution
                            .insert(*id, Deserialize::deserialize(record)?);
                    }

                    "VERTEX_POINT" => {
                        self.vertex_point
                            .insert(*id, Deserialize::deserialize(record)?);
                    }
                    "EDGE_CURVE" => {
                        self.edge_curve
                            .insert(*id, Deserialize::deserialize(record)?);
                    }
                    "ORIENTED_EDGE" => {
                        if let Parameter::List(params) = &record.parameter {
                            if params.len() == 5 {
                                self.oriented_edge.insert(
                                    *id,
                                    OrientedEdgeHolder {
                                        label: Deserialize::deserialize(&params[0])?,
                                        edge_element: Deserialize::deserialize(&params[3])?,
                                        orientation: Deserialize::deserialize(&params[4])?,
                                    },
                                );
                            }
                        }
                    }
                    "EDGE_LOOP" => {
                        self.edge_loop
                            .insert(*id, Deserialize::deserialize(record)?);
                    }
                    "VERTEX_LOOP" => {
                        self.vertex_loop
                            .insert(*id, Deserialize::deserialize(record)?);
                    }
                    "FACE_BOUND" => {
                        self.face_bound
                            .insert(*id, Deserialize::deserialize(record)?);
                    }
                    "FACE_OUTER_BOUND" => {
                        self.face_bound
                            .insert(*id, Deserialize::deserialize(&record.parameter)?);
                        self.face_outer_bound_ids.insert(*id);
                    }
                    "FACE_SURFACE" => {
                        self.face_surface
                            .insert(*id, Deserialize::deserialize(record)?);
                    }
                    "ADVANCED_FACE" => {
                        self.face_surface
                            .insert(*id, Deserialize::deserialize(&record.parameter)?);
                    }
                    "ORIENTED_FACE" => {
                        if let Parameter::List(params) = &record.parameter {
                            if params.len() == 4 {
                                self.oriented_face.insert(
                                    *id,
                                    OrientedFaceHolder {
                                        label: Deserialize::deserialize(&params[0])?,
                                        face_element: Deserialize::deserialize(&params[2])?,
                                        orientation: Deserialize::deserialize(&params[3])?,
                                    },
                                );
                            }
                        }
                    }
                    "OPEN_SHELL" => {
                        self.shell
                            .insert(*id, Deserialize::deserialize(&record.parameter)?);
                    }
                    "CLOSED_SHELL" => {
                        self.shell
                            .insert(*id, Deserialize::deserialize(&record.parameter)?);
                    }
                    "ORIENTED_OPEN_SHELL" => {
                        if let Parameter::List(params) = &record.parameter {
                            if params.len() == 4 {
                                self.oriented_shell.insert(
                                    *id,
                                    OrientedShellHolder {
                                        label: Deserialize::deserialize(&params[0])?,
                                        shell_element: Deserialize::deserialize(&params[2])?,
                                        orientation: Deserialize::deserialize(&params[3])?,
                                    },
                                );
                            }
                        }
                    }
                    "ORIENTED_CLOSED_SHELL" => {
                        if let Parameter::List(params) = &record.parameter {
                            if params.len() == 4 {
                                self.oriented_shell.insert(
                                    *id,
                                    OrientedShellHolder {
                                        label: Deserialize::deserialize(&params[0])?,
                                        shell_element: Deserialize::deserialize(&params[2])?,
                                        orientation: Deserialize::deserialize(&params[3])?,
                                    },
                                );
                            }
                        }
                    }
                    "SHELL_BASED_SURFACE_MODEL" => {
                        self.shell_based_surface_model
                            .insert(*id, Deserialize::deserialize(&record.parameter)?);
                    }
                    "MANIFOLD_SOLID_BREP" => {
                        if let Parameter::List(params) = &record.parameter {
                            if params.len() == 2 {
                                self.manifold_solid_brep.insert(
                                    *id,
                                    ManifoldSolidBrepHolder {
                                        label: Deserialize::deserialize(&params[0])?,
                                        outer: Deserialize::deserialize(&params[1])?,
                                        voids: Vec::new(),
                                    },
                                );
                            }
                        }
                    }
                    "BREP_WITH_VOIDS" => {
                        self.manifold_solid_brep
                            .insert(*id, Deserialize::deserialize(&record.parameter)?);
                    }
                    "DEFINITIONAL_REPRESENTATION" => {
                        if let Parameter::List(params) = &record.parameter {
                            if params.len() == 3 {
                                self.definitional_representation.insert(
                                    *id,
                                    DefinitionalRepresentationHolder {
                                        label: Deserialize::deserialize(&params[0])?,
                                        representation_item: Deserialize::deserialize(&params[1])?,
                                        context_of_items: match &params[2] {
                                            Parameter::Ref(x) => PlaceHolder::Ref(x.clone()),
                                            _ => PlaceHolder::Owned(DummyHolder {
                                                record: format!("{:?}", params[2]),
                                                is_simple: true,
                                            }),
                                        },
                                    },
                                );
                            }
                        }
                    }
                    "APPLICATION_CONTEXT" => {
                        self.application_context
                            .insert(*id, Deserialize::deserialize(&record.parameter)?);
                    }
                    "PRODUCT_CONTEXT" => {
                        self.product_context
                            .insert(*id, Deserialize::deserialize(&record.parameter)?);
                    }
                    "PRODUCT" => {
                        self.product
                            .insert(*id, Deserialize::deserialize(&record.parameter)?);
                    }
                    "PRODUCT_DEFINITION_FORMATION" => {
                        self.product_definition_formation
                            .insert(*id, Deserialize::deserialize(&record.parameter)?);
                    }
                    "PRODUCT_DEFINITION_FORMATION_WITH_SPECIFIED_SOURCE" => {
                        if let Parameter::List(params) = &record.parameter {
                            if params.len() >= 3 {
                                self.product_definition_formation.insert(
                                    *id,
                                    ProductDefinitionFormationHolder {
                                        id: Deserialize::deserialize(&params[0])?,
                                        description: Deserialize::deserialize(&params[1])?,
                                        of_product: Deserialize::deserialize(&params[2])?,
                                    },
                                );
                            }
                        }
                    }
                    "PRODUCT_DEFINITION_CONTEXT" => {
                        self.product_definition_context
                            .insert(*id, Deserialize::deserialize(&record.parameter)?);
                    }
                    "PRODUCT_DEFINITION" => {
                        self.product_definition
                            .insert(*id, Deserialize::deserialize(&record.parameter)?);
                    }
                    "PRODUCT_DEFINITION_SHAPE" => {
                        self.product_definition_shape
                            .insert(*id, Deserialize::deserialize(&record.parameter)?);
                    }
                    "SHAPE_DEFINITION_REPRESENTATION" => {
                        self.shape_definition_representation
                            .insert(*id, Deserialize::deserialize(&record.parameter)?);
                    }
                    "SHAPE_REPRESENTATION" => {
                        self.shape_representation
                            .insert(*id, Deserialize::deserialize(&record.parameter)?);
                    }
                    "ADVANCED_BREP_SHAPE_REPRESENTATION" => {
                        self.shape_representation
                            .insert(*id, Deserialize::deserialize(&record.parameter)?);
                    }
                    "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION" => {
                        self.context_dependent_shape_representation
                            .insert(*id, Deserialize::deserialize(&record.parameter)?);
                    }
                    "SHAPE_REPRESENTATION_RELATIONSHIP" => {
                        self.shape_representation_relationship
                            .insert(*id, Deserialize::deserialize(&record.parameter)?);
                    }
                    "NEXT_ASSEMBLY_USAGE_OCCURRENCE" => {
                        self.next_assembly_usage_occurrence
                            .insert(*id, Deserialize::deserialize(&record.parameter)?);
                    }
                    "ITEM_DEFINED_TRANSFORMATION" => {
                        self.item_defined_transformation
                            .insert(*id, Deserialize::deserialize(&record.parameter)?);
                    }
                    _ => {
                        self.dummy.insert(
                            *id,
                            DummyHolder {
                                record: format!("{record:?}"),
                                is_simple: true,
                            },
                        );
                    }
                }
            }
            EntityInstance::Complex {
                id,
                subsuper: SubSuperRecord(records),
            } => {
                use NonRationalBSplineCurveHolder as NRBC;
                use NonRationalBSplineSurfaceHolder as NRBS;
                if records.len() == 7 {
                    match (
                        records[0].name.as_str(),
                        &records[0].parameter,
                        records[1].name.as_str(),
                        &records[1].parameter,
                        records[2].name.as_str(),
                        &records[2].parameter,
                        records[3].name.as_str(),
                        &records[3].parameter,
                        records[4].name.as_str(),
                        &records[4].parameter,
                        records[5].name.as_str(),
                        &records[5].parameter,
                        records[6].name.as_str(),
                        &records[6].parameter,
                    ) {
                        (
                            "BOUNDED_CURVE",
                            _,
                            "B_SPLINE_CURVE",
                            Parameter::List(bsp_params),
                            "B_SPLINE_CURVE_WITH_KNOTS",
                            Parameter::List(knots_params),
                            "CURVE",
                            _,
                            "GEOMETRIC_REPRESENTATION_ITEM",
                            _,
                            "RATIONAL_B_SPLINE_CURVE",
                            Parameter::List(weights),
                            "REPRESENTATION_ITEM",
                            Parameter::List(label),
                        ) => {
                            let mut params = label.clone();
                            params.extend(bsp_params.clone());
                            params.extend(knots_params.clone());
                            self.rational_b_spline_curve.insert(
                                *id,
                                RationalBSplineCurveHolder {
                                    non_rational_b_spline_curve: PlaceHolder::Owned(
                                        NRBC::BSplineCurveWithKnots(Deserialize::deserialize(
                                            &Parameter::List(params),
                                        )?),
                                    ),
                                    weights_data: Deserialize::deserialize(&weights[0])?,
                                },
                            );
                        }
                        (
                            "BEZIER_CURVE",
                            _,
                            "BOUNDED_CURVE",
                            _,
                            "B_SPLINE_CURVE",
                            Parameter::List(bsp_params),
                            "CURVE",
                            _,
                            "GEOMETRIC_REPRESENTATION_ITEM",
                            _,
                            "RATIONAL_B_SPLINE_CURVE",
                            Parameter::List(weights),
                            "REPRESENTATION_ITEM",
                            Parameter::List(label),
                        ) => {
                            let mut params = label.clone();
                            params.extend(bsp_params.clone());
                            self.rational_b_spline_curve.insert(
                                *id,
                                RationalBSplineCurveHolder {
                                    non_rational_b_spline_curve: PlaceHolder::Owned(
                                        NRBC::BezierCurve(Deserialize::deserialize(
                                            &Parameter::List(params),
                                        )?),
                                    ),
                                    weights_data: Deserialize::deserialize(&weights[0])?,
                                },
                            );
                        }
                        (
                            "BOUNDED_CURVE",
                            _,
                            "B_SPLINE_CURVE",
                            Parameter::List(bsp_params),
                            "CURVE",
                            _,
                            "GEOMETRIC_REPRESENTATION_ITEM",
                            _,
                            "QUASI_UNIFORM_CURVE",
                            _,
                            "RATIONAL_B_SPLINE_CURVE",
                            Parameter::List(weights),
                            "REPRESENTATION_ITEM",
                            Parameter::List(label),
                        ) => {
                            let mut params = vec![label[0].clone()];
                            params.extend(bsp_params.iter().cloned());
                            self.rational_b_spline_curve.insert(
                                *id,
                                RationalBSplineCurveHolder {
                                    non_rational_b_spline_curve: PlaceHolder::Owned(
                                        NRBC::QuasiUniformCurve(Deserialize::deserialize(
                                            &Parameter::List(params),
                                        )?),
                                    ),
                                    weights_data: Deserialize::deserialize(&weights[0])?,
                                },
                            );
                        }
                        (
                            "BOUNDED_CURVE",
                            _,
                            "B_SPLINE_CURVE",
                            Parameter::List(bsp_params),
                            "CURVE",
                            _,
                            "GEOMETRIC_REPRESENTATION_ITEM",
                            _,
                            "RATIONAL_B_SPLINE_CURVE",
                            Parameter::List(weights),
                            "REPRESENTATION_ITEM",
                            Parameter::List(label),
                            "UNIFORM_CURVE",
                            _,
                        ) => {
                            let mut params = vec![label[0].clone()];
                            params.extend(bsp_params.iter().cloned());
                            self.rational_b_spline_curve.insert(
                                *id,
                                RationalBSplineCurveHolder {
                                    non_rational_b_spline_curve: PlaceHolder::Owned(
                                        NRBC::UniformCurve(Deserialize::deserialize(
                                            &Parameter::List(params),
                                        )?),
                                    ),
                                    weights_data: Deserialize::deserialize(&weights[0])?,
                                },
                            );
                        }
                        (
                            "BOUNDED_SURFACE",
                            _,
                            "B_SPLINE_SURFACE",
                            Parameter::List(bsp_params),
                            "B_SPLINE_SURFACE_WITH_KNOTS",
                            Parameter::List(knots_params),
                            "GEOMETRIC_REPRESENTATION_ITEM",
                            _,
                            "RATIONAL_B_SPLINE_SURFACE",
                            Parameter::List(weights),
                            "REPRESENTATION_ITEM",
                            Parameter::List(label),
                            "SURFACE",
                            _,
                        ) => {
                            let mut params = label.clone();
                            params.extend(bsp_params.clone());
                            params.extend(knots_params.clone());
                            self.rational_b_spline_surface.insert(
                                *id,
                                RationalBSplineSurfaceHolder {
                                    non_rational_b_spline_surface: PlaceHolder::Owned(
                                        NRBS::BSplineSurfaceWithKnots(Deserialize::deserialize(
                                            &Parameter::List(params),
                                        )?),
                                    ),
                                    weights_data: Deserialize::deserialize(&weights[0])?,
                                },
                            );
                        }
                        (
                            "BEZIER_SURFACE",
                            _,
                            "BOUNDED_SURFACE",
                            _,
                            "B_SPLINE_SURFACE",
                            Parameter::List(bsp_params),
                            "GEOMETRIC_REPRESENTATION_ITEM",
                            _,
                            "RATIONAL_B_SPLINE_SURFACE",
                            Parameter::List(weights),
                            "REPRESENTATION_ITEM",
                            Parameter::List(label),
                            "SURFACE",
                            _,
                        ) => {
                            let mut params = label.clone();
                            params.extend(bsp_params.clone());
                            self.rational_b_spline_surface.insert(
                                *id,
                                RationalBSplineSurfaceHolder {
                                    non_rational_b_spline_surface: PlaceHolder::Owned(
                                        NRBS::BezierSurface(Deserialize::deserialize(
                                            &Parameter::List(params),
                                        )?),
                                    ),
                                    weights_data: Deserialize::deserialize(&weights[0])?,
                                },
                            );
                        }
                        (
                            "BOUNDED_SURFACE",
                            _,
                            "B_SPLINE_SURFACE",
                            Parameter::List(bsp_params),
                            "GEOMETRIC_REPRESENTATION_ITEM",
                            _,
                            "QUASI_UNIFORM_SURFACE",
                            _,
                            "RATIONAL_B_SPLINE_SURFACE",
                            Parameter::List(weights),
                            "REPRESENTATION_ITEM",
                            Parameter::List(label),
                            "SURFACE",
                            _,
                        ) => {
                            let mut params = label.clone();
                            params.extend(bsp_params.clone());
                            self.rational_b_spline_surface.insert(
                                *id,
                                RationalBSplineSurfaceHolder {
                                    non_rational_b_spline_surface: PlaceHolder::Owned(
                                        NRBS::QuasiUniformSurface(Deserialize::deserialize(
                                            &Parameter::List(params),
                                        )?),
                                    ),
                                    weights_data: Deserialize::deserialize(&weights[0])?,
                                },
                            );
                        }
                        (
                            "BOUNDED_SURFACE",
                            _,
                            "B_SPLINE_SURFACE",
                            Parameter::List(bsp_params),
                            "GEOMETRIC_REPRESENTATION_ITEM",
                            _,
                            "RATIONAL_B_SPLINE_SURFACE",
                            Parameter::List(weights),
                            "REPRESENTATION_ITEM",
                            Parameter::List(label),
                            "SURFACE",
                            _,
                            "UNIFORM_SURFACE",
                            _,
                        ) => {
                            let mut params = label.clone();
                            params.extend(bsp_params.clone());
                            self.rational_b_spline_surface.insert(
                                *id,
                                RationalBSplineSurfaceHolder {
                                    non_rational_b_spline_surface: PlaceHolder::Owned(
                                        NRBS::UniformSurface(Deserialize::deserialize(
                                            &Parameter::List(params),
                                        )?),
                                    ),
                                    weights_data: Deserialize::deserialize(&weights[0])?,
                                },
                            );
                        }
                        _ => {
                            self.dummy.insert(
                                *id,
                                DummyHolder {
                                    record: format!("{records:?}"),
                                    is_simple: false,
                                },
                            );
                        }
                    }
                } else if records.len() == 3 {
                    match (
                        records[0].name.as_str(),
                        &records[0].parameter,
                        records[1].name.as_str(),
                        &records[1].parameter,
                        records[2].name.as_str(),
                        &records[2].parameter,
                    ) {
                        (
                            "REPRESENTATION_RELATIONSHIP",
                            Parameter::List(rr_parameter),
                            "REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION",
                            Parameter::List(transformation),
                            "SHAPE_REPRESENTATION_RELATIONSHIP",
                            _,
                        ) => {
                            let entity = ShapeRepresentationRelationshipWithTransformationHolder {
                                name: Deserialize::deserialize(&rr_parameter[0])?,
                                description: Deserialize::deserialize(&rr_parameter[1])?,
                                rep_1: Deserialize::deserialize(&rr_parameter[2])?,
                                rep_2: Deserialize::deserialize(&rr_parameter[3])?,
                                transformation_operator: Deserialize::deserialize(
                                    &transformation[0],
                                )?,
                            };
                            self.shape_representation_relationship_with_transformation
                                .insert(*id, entity);
                        }
                        _ => {
                            self.dummy.insert(
                                *id,
                                DummyHolder {
                                    record: format!("{records:?}"),
                                    is_simple: false,
                                },
                            );
                        }
                    }
                } else {
                    self.dummy.insert(
                        *id,
                        DummyHolder {
                            record: format!("{records:?}"),
                            is_simple: false,
                        },
                    );
                }
            }
        }
        self.collect_plane_angle_unit(instance);
        self.collect_geometric_uncertainty(instance);
        Ok(())
    }

    /// Record the file's plane-angle unit declarations as they go past.
    ///
    /// Separate from the main dispatch above rather than folded into it: that
    /// match is six hundred lines of geometry and this is bookkeeping about how
    /// to read a number, not another entity to convert.
    fn collect_plane_angle_unit(&mut self, instance: &EntityInstance) {
        match instance {
            EntityInstance::Simple { id, record } => {
                // `PLANE_ANGLE_MEASURE_WITH_UNIT(PLANE_ANGLE_MEASURE(0.01745), #18)`
                // — the factor is radians per unit of the converted unit.
                if record.name == "PLANE_ANGLE_MEASURE_WITH_UNIT" {
                    if let Parameter::List(params) = &record.parameter {
                        if let Some(Parameter::Typed { parameter, .. }) = params.first() {
                            if let Parameter::Real(value) = **parameter {
                                let base = match params.get(1) {
                                    Some(Parameter::Ref(Name::Entity(unit))) => Some(*unit),
                                    _ => None,
                                };
                                self.plane_angle_measures.insert(*id, (value, base));
                            }
                        }
                    }
                }
            }
            EntityInstance::Complex {
                id,
                subsuper: SubSuperRecord(records),
            } => {
                // A unit is a complex instance: the supertypes are listed side
                // by side, and `PLANE_ANGLE_UNIT` among them is what makes this
                // an angle unit rather than a length or solid-angle one.
                if !records.iter().any(|r| r.name == "PLANE_ANGLE_UNIT") {
                    return;
                }
                if let Some(cbu) = records.iter().find(|r| r.name == "CONVERSION_BASED_UNIT") {
                    if let Parameter::List(params) = &cbu.parameter {
                        // CONVERSION_BASED_UNIT(name, conversion_factor)
                        if let Some(Parameter::Ref(Name::Entity(measure))) = params.get(1) {
                            self.plane_angle_units
                                .push((*id, PlaneAngleUnit::Converted { measure: *measure }));
                            return;
                        }
                    }
                }
                if records.iter().any(|r| r.name == "SI_UNIT") {
                    self.plane_angle_units.push((*id, PlaneAngleUnit::Radian));
                }
            }
        }
    }

    /// Record the file's geometric-uncertainty declarations as they go past.
    ///
    /// Separate from the main dispatch above for the same reason the plane-angle
    /// collector is: this is bookkeeping about how to read a number that other
    /// entities reference, not an entity to convert.
    fn collect_geometric_uncertainty(&mut self, instance: &EntityInstance) {
        match instance {
            EntityInstance::Simple { id, record } => {
                // `UNCERTAINTY_MEASURE_WITH_UNIT(value, unit, name, description)`.
                // The value is written either bare or inside a typed length
                // measure -- both `#u=UNCERTAINTY_MEASURE_WITH_UNIT(1.0E-6, ...)`
                // and `#u=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.005), ...)`
                // occur in the wild -- and it is always in the model's native
                // length unit.
                if record.name == "UNCERTAINTY_MEASURE_WITH_UNIT" {
                    if let Parameter::List(params) = &record.parameter {
                        let value = params.first().and_then(|value| match value {
                            Parameter::Real(value) => Some(*value),
                            Parameter::Typed { parameter, .. } => match &**parameter {
                                Parameter::Real(value) => Some(*value),
                                _ => None,
                            },
                            _ => None,
                        });
                        if let Some(value) = value {
                            self.uncertainty_measures.insert(*id, value);
                        }
                    }
                }
            }
            EntityInstance::Complex {
                id,
                subsuper: SubSuperRecord(records),
            } => {
                // A `GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#u1, #u2, ...))` is a
                // supertype of `REPRESENTATION_CONTEXT`, so a context that
                // declares an uncertainty is a complex entity. The assignment
                // record names the uncertainty measures; the value itself is a
                // simple `UNCERTAINTY_MEASURE_WITH_UNIT` recorded above, and
                // the two are joined by reference because entity order in the
                // file is not a guarantee of anything.
                for record in records {
                    if record.name != "GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT" {
                        continue;
                    }
                    let mut measures = Vec::new();
                    if let Parameter::List(params) = &record.parameter {
                        for set in params {
                            if let Parameter::List(members) = set {
                                for member in members {
                                    if let Parameter::Ref(Name::Entity(measure)) = member {
                                        measures.push(*measure);
                                    }
                                }
                            }
                        }
                    }
                    if !measures.is_empty() {
                        self.global_uncertainty_assigned_contexts
                            .insert(*id, measures);
                    }
                }
            }
        }
    }

    /// The declared geometric uncertainty of the shape representation that owns
    /// `shell_id`, in the file's native length units.
    ///
    /// Resolves the chain
    ///
    /// ```text
    /// shell → owning solid/shell model → shape representation
    ///     → representation context → GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT
    ///     → UNCERTAINTY_MEASURE_WITH_UNIT
    /// ```
    ///
    /// and returns the first finite, positive value, or `None` when the shell is
    /// not owned by any representation, the representation declares no
    /// uncertainty, or the declared value is unusable. `None` is the honest
    /// answer to "the source supplies no usable uncertainty" -- the caller then
    /// falls back to its own numerical tolerance rather than inventing one.
    pub fn source_geometric_uncertainty(&self, shell_id: u64) -> Option<f64> {
        use PlaceHolder::Ref;
        // The shell id is what the owning solid's `outer` (or a shell model's
        // boundary) names. Find a solid or shell model that references it.
        let solid_id = self
            .manifold_solid_brep
            .iter()
            .find_map(|(&id, solid)| {
                self.shell_entity_id(&solid.outer)
                    .filter(|outer| *outer == shell_id)
                    .map(|_| id)
            })
            .or_else(|| {
                self.shell_based_surface_model
                    .iter()
                    .find_map(|(&id, model)| {
                        model
                            .sbsm_boundary
                            .iter()
                            .any(|boundary| {
                                self.shell_entity_id(boundary)
                                    .map_or(false, |b| b == shell_id)
                            })
                            .then_some(id)
                    })
            })?;
        // The shape representation whose items name that solid or model.
        let representation = self.shape_representation.values().find(|sr| {
            sr.items.iter().any(|item| {
                if let Ref(Name::Entity(item_id)) = item {
                    *item_id == solid_id
                } else {
                    false
                }
            })
        })?;
        // The context the representation declares, resolved to an entity id.
        let Ref(Name::Entity(context_id)) = &representation.context_of_items else {
            return None;
        };
        // The uncertainty assigned to that context.
        let measures = self.global_uncertainty_assigned_contexts.get(context_id)?;
        measures.iter().find_map(|measure_id| {
            let value = self.uncertainty_measures.get(measure_id)?;
            value.is_finite().then_some(*value).filter(|v| *v > 0.0)
        })
    }

    /// The entity id of the shell a solid's `outer` or a shell model's boundary
    /// names, following an oriented-shell indirection when present.
    fn shell_entity_id(&self, shell_any: &PlaceHolder<ShellAnyHolder>) -> Option<u64> {
        use PlaceHolder::Ref;
        let Ref(Name::Entity(id)) = shell_any else {
            return None;
        };
        if self.shell.contains_key(id) {
            return Some(*id);
        }
        let oriented = self.oriented_shell.get(id)?;
        let Ref(Name::Entity(element)) = &oriented.shell_element else {
            return None;
        };
        Some(*element)
    }

    /// Radians per unit for the angles in this file, or 1 if that is unknowable.
    ///
    /// **Resolved file-globally, and only when every declaration agrees.** The
    /// strictly correct rule is per-representation: a shell's angles are read in
    /// the units its `GEOMETRIC_REPRESENTATION_CONTEXT` assigns, and a file may
    /// carry several contexts — `ftc_07` has two. Associating shells with their
    /// representation is more machinery than the measured defect needs, because
    /// in that file *both* contexts declare degrees.
    ///
    /// So the rule here is deliberately narrow: agree, or do nothing. A file
    /// that really does mix radians for geometry with degrees for annotation
    /// gets a warning and no conversion, which leaves it exactly as wrong as it
    /// was before rather than newly wrong in a different way. Guessing which
    /// context owns the geometry is how a fix for one file breaks twenty.
    pub fn plane_angle_factor(&self) -> f64 {
        // Units that exist only to define another unit are not declarations of
        // how this file writes its angles. `DEGREE` is defined as a multiple of
        // a radian `SI_UNIT`, so that radian unit appears in every degree file
        // and must not be counted as disagreeing with the degree unit it
        // defines. Without this the rule below refuses every file it exists to
        // fix — which is exactly what it did on the first run.
        let bases: Vec<u64> = self
            .plane_angle_measures
            .values()
            .filter_map(|(_, base)| *base)
            .collect();

        let mut agreed: Option<f64> = None;
        for (id, unit) in &self.plane_angle_units {
            if bases.contains(id) {
                continue;
            }
            let factor = match unit {
                PlaneAngleUnit::Radian => 1.0,
                PlaneAngleUnit::Converted { measure } => {
                    match self.plane_angle_measures.get(measure) {
                        Some((value, _)) if value.is_finite() && *value > 0.0 => *value,
                        // A conversion unit whose factor did not resolve is not
                        // an invitation to assume radians; it is unknown.
                        _ => return 1.0,
                    }
                }
            };
            match agreed {
                None => agreed = Some(factor),
                Some(seen) if (seen - factor).abs() <= 1.0e-12 * seen.abs().max(1.0) => {}
                Some(seen) => {
                    eprintln!(
                        "plane angle units disagree ({seen} vs {factor} radians per unit); \
                         angles left unconverted"
                    );
                    return 1.0;
                }
            }
        }
        agreed.unwrap_or(1.0)
    }

    /// Convert every angle-valued attribute into radians.
    ///
    /// Done once, on the table, rather than threaded through conversion: the
    /// geometry conversions are `From` impls with no access to the table, and
    /// giving them one would touch every surface and curve type to fix a defect
    /// measured in exactly one attribute.
    ///
    /// `CONICAL_SURFACE.semi_angle` is the attribute proven to matter — it is
    /// what turned `ftc_07`'s corner fillets into fans. **`PARAMETER_VALUE`
    /// trims on circles are also angle-valued and are not handled here**;
    /// `ftc_07` contains none, so they are outside what the reproducer
    /// demonstrates, but 20 of the 33 NIST files do contain them and `ctc_05`
    /// is one. Expect to come back here for that.
    fn normalize_angle_units(&mut self) {
        let factor = self.plane_angle_factor();
        if (factor - 1.0).abs() < f64::EPSILON {
            return;
        }
        for cone in self.conical_surface.values_mut() {
            cone.semi_angle *= factor;
        }
    }

    #[inline(always)]
    pub fn from_data_section(data_section: &DataSection) -> Table {
        Table::from_iter(&data_section.entities)
    }

    /// Build a table from a data section it takes ownership of.
    ///
    /// [`Self::from_data_section`] borrows, so the syntax tree stays fully
    /// resident while the table is filled and a large model pays for both
    /// representations at once. On a hundred-megabyte assembly the tree is
    /// around eight times the file and the table another three, so holding
    /// both is most of the peak.
    ///
    /// This consumes each entity as it is converted, letting its storage be
    /// reused by the table being built rather than sitting on it until the
    /// end. The resulting table is identical either way; only the high-water
    /// mark differs.
    #[inline(always)]
    pub fn from_owned_data_section(data_section: DataSection) -> Table {
        Table::from_iter(data_section.entities)
    }
    #[inline(always)]
    pub fn from_step(step_str: &str) -> Option<Table> {
        let exchange = ruststep::parser::parse(step_str).ok()?;
        Some(Table::from_data_section(&exchange.data[0]))
    }
}

impl<'a> FromIterator<&'a EntityInstance> for Table {
    fn from_iter<I: IntoIterator<Item = &'a EntityInstance>>(iter: I) -> Table {
        let mut res = Table::default();
        iter.into_iter().for_each(|instance| {
            res.push_instance(instance)
                .unwrap_or_else(|e| eprintln!("{e}"))
        });
        // Units are resolved here rather than in the three public constructors
        // because every one of them funnels through this trait. Normalising at
        // the public entry points instead would leave a direct `from_iter` call
        // holding a table whose angles are in whatever the file happened to
        // use — a hole that only shows up on a file with non-radian angles,
        // which is exactly the case this exists for.
        res.normalize_angle_units();
        res
    }
}

/// Consuming counterpart of the borrowing implementation above.
///
/// Each entity is dropped as soon as it has been pushed, so the allocator can
/// hand its storage straight back to the table instead of the caller holding a
/// whole second copy of the model until the table is finished.
impl FromIterator<EntityInstance> for Table {
    fn from_iter<I: IntoIterator<Item = EntityInstance>>(iter: I) -> Table {
        let mut res = Table::default();
        iter.into_iter().for_each(|instance| {
            res.push_instance(&instance)
                .unwrap_or_else(|e| eprintln!("{e}"))
        });
        res.normalize_angle_units();
        res
    }
}

/// Undefined structures are parsed into this.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = dummy)]
#[holder(generate_deserialize)]
pub struct Dummy {
    pub record: String,
    pub is_simple: bool,
}

/// Many geometric and topological elements are contained within this entity's child classes.
/// Since it is essentially an `Any` type, one must manually map the reference according to the context.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = representation_item)]
#[holder(generate_deserialize)]
pub struct RepresentationItem {
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = representation_context)]
#[holder(generate_deserialize)]
pub struct RepresentationContext {
    pub context_identifier: String,
    pub context_type: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = representation)]
#[holder(generate_deserialize)]
pub struct Representation {
    pub name: String,
    #[holder(use_place_holder)]
    pub items: Vec<RepresentationItem>,
    #[holder(use_place_holder)]
    pub context_of_items: Vec<RepresentationContext>,
}

/// `cartesian_point`
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = cartesian_point)]
#[holder(generate_deserialize)]
pub struct CartesianPoint {
    pub label: String,
    pub coordinates: Vec<f64>,
}
impl From<&CartesianPoint> for Point2 {
    #[inline(always)]
    fn from(pt: &CartesianPoint) -> Self {
        let pt = &pt.coordinates;
        match pt.len() {
            0 => Point2::origin(),
            1 => Point2::new(pt[0], 0.0),
            _ => Point2::new(pt[0], pt[1]),
        }
    }
}
impl From<&CartesianPoint> for Point3 {
    #[inline(always)]
    fn from(pt: &CartesianPoint) -> Self {
        let pt = &pt.coordinates;
        match pt.len() {
            0 => Point3::origin(),
            1 => Point3::new(pt[0], 0.0, 0.0),
            2 => Point3::new(pt[0], pt[1], 0.0),
            _ => Point3::new(pt[0], pt[1], pt[2]),
        }
    }
}

/// `direction`
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = direction)]
#[holder(generate_deserialize)]
pub struct Direction {
    pub label: String,
    pub direction_ratios: Vec<f64>,
}
impl From<&Direction> for Vector2 {
    #[inline(always)]
    fn from(dir: &Direction) -> Self {
        let dir = &dir.direction_ratios;
        match dir.len() {
            0 => Vector2::zero(),
            1 => Vector2::new(dir[0], 0.0),
            _ => Vector2::new(dir[0], dir[1]),
        }
    }
}
impl From<&Direction> for Vector3 {
    #[inline(always)]
    fn from(dir: &Direction) -> Self {
        let dir = &dir.direction_ratios;
        match dir.len() {
            0 => Vector3::zero(),
            1 => Vector3::new(dir[0], 0.0, 0.0),
            2 => Vector3::new(dir[0], dir[1], 0.0),
            _ => Vector3::new(dir[0], dir[1], dir[2]),
        }
    }
}

/// `vector`
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = vector)]
#[holder(generate_deserialize)]
pub struct Vector {
    pub label: String,
    #[holder(use_place_holder)]
    pub orientation: Direction,
    pub magnitude: f64,
}
impl From<&Vector> for Vector2 {
    #[inline(always)]
    fn from(vec: &Vector) -> Self {
        Self::from(&vec.orientation) * vec.magnitude
    }
}
impl From<&Vector> for Vector3 {
    #[inline(always)]
    fn from(vec: &Vector) -> Self {
        Self::from(&vec.orientation) * vec.magnitude
    }
}

/// `placement`
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = placement)]
#[holder(generate_deserialize)]
pub struct Placement {
    pub label: String,
    #[holder(use_place_holder)]
    pub location: CartesianPoint,
}
impl From<&Placement> for Point2 {
    #[inline(always)]
    fn from(p: &Placement) -> Self {
        Self::from(&p.location)
    }
}
impl From<&Placement> for Point3 {
    #[inline(always)]
    fn from(p: &Placement) -> Self {
        Self::from(&p.location)
    }
}

/// `axis1_placement`
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = axis1_placement)]
#[holder(generate_deserialize)]
pub struct Axis1Placement {
    pub label: String,
    #[holder(use_place_holder)]
    pub location: CartesianPoint,
    #[holder(use_place_holder)]
    pub direction: Option<Direction>,
}

impl Axis1Placement {
    pub fn direction(&self) -> Vector3 {
        self.direction
            .as_ref()
            .map(Vector3::from)
            .unwrap_or_else(Vector3::unit_z)
    }
}

/// `axis2_placement`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(generate_deserialize)]
pub enum Axis2Placement {
    #[holder(use_place_holder)]
    Axis2Placement2d(Axis2Placement2d),
    #[holder(use_place_holder)]
    Axis2Placement3d(Axis2Placement3d),
}

impl TryFrom<&Axis2Placement> for Matrix3 {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(axis: &Axis2Placement) -> Result<Self, StepConvertingError> {
        use Axis2Placement::*;
        match axis {
            Axis2Placement2d(axis) => Ok(Matrix3::from(axis)),
            Axis2Placement3d(_) => Err("This is not a 2D axis placement.".into()),
        }
    }
}
impl TryFrom<&Axis2Placement> for Matrix4 {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(axis: &Axis2Placement) -> Result<Self, StepConvertingError> {
        use Axis2Placement::*;
        match axis {
            Axis2Placement2d(_) => Err("This is not a 3D axis placement.".into()),
            Axis2Placement3d(axis) => Ok(Matrix4::from(axis)),
        }
    }
}

/// `axis2_placement_2d`
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = axis2_placement_2d)]
#[holder(generate_deserialize)]
pub struct Axis2Placement2d {
    pub label: String,
    #[holder(use_place_holder)]
    pub location: CartesianPoint,
    #[holder(use_place_holder)]
    pub ref_direction: Option<Direction>,
}

impl From<&Axis2Placement2d> for Matrix3 {
    #[inline(always)]
    fn from(axis: &Axis2Placement2d) -> Self {
        let z = Point2::from(&axis.location);
        let x = match &axis.ref_direction {
            Some(axis) => Vector2::from(axis),
            None => Vector2::unit_x(),
        };
        let y = Vector2::new(-x.y, x.x);
        Matrix3::from_cols(x.extend(0.0), y.extend(0.0), z.to_vec().extend(1.0))
    }
}

/// `axis2_placement_3d`
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = axis2_placement_3d)]
#[holder(generate_deserialize)]
pub struct Axis2Placement3d {
    pub label: String,
    #[holder(use_place_holder)]
    pub location: CartesianPoint,
    #[holder(use_place_holder)]
    pub axis: Option<Direction>,
    #[holder(use_place_holder)]
    pub ref_direction: Option<Direction>,
}

/// Normalize, or report that the vector carries no usable direction.
///
/// `DIRECTION` only has to be non-zero in a conforming file, and it does not
/// have to be unit length. Exporters emit both zero-length and non-unit
/// directions, so every direction read from STEP goes through here.
#[inline(always)]
fn normalized_or_none(vector: Vector3) -> Option<Vector3> {
    let magnitude = vector.magnitude();
    (magnitude.is_finite() && magnitude > f64::EPSILON).then(|| vector / magnitude)
}

/// Any unit vector perpendicular to `axis`, used when a placement does not
/// supply a usable reference direction.
///
/// Crossing with the least-aligned cardinal axis keeps the result well
/// conditioned no matter how `axis` is oriented.
#[inline(always)]
fn any_perpendicular(axis: Vector3) -> Vector3 {
    let (x, y, z) = (axis.x.abs(), axis.y.abs(), axis.z.abs());
    let cardinal = if x <= y && x <= z {
        Vector3::unit_x()
    } else if y <= z {
        Vector3::unit_y()
    } else {
        Vector3::unit_z()
    };
    normalized_or_none(cardinal.cross(axis)).unwrap_or_else(Vector3::unit_x)
}

impl From<&Axis2Placement3d> for Matrix4 {
    #[inline(always)]
    fn from(axis: &Axis2Placement3d) -> Matrix4 {
        let w = Point3::from(&axis.location);
        let z = match &axis.axis {
            Some(axis) => Vector3::from(axis),
            None => Vector3::unit_z(),
        };
        // A zero or non-unit axis would otherwise produce a singular basis,
        // which only fails much later and far from the cause.
        let z = normalized_or_none(z).unwrap_or_else(Vector3::unit_z);
        let x = match &axis.ref_direction {
            Some(axis) => Vector3::from(axis),
            None => Vector3::unit_x(),
        };
        // Project the reference direction into the plane normal to the axis.
        // When the two are parallel this leaves the zero vector, and
        // normalizing that yields NaN that propagates silently through every
        // curve and surface built on this placement, surfacing later as an
        // unrelated tolerance assertion. Any perpendicular is a valid basis
        // here, because the reference direction carried no usable information.
        let x = normalized_or_none(x - x.dot(z) * z).unwrap_or_else(|| any_perpendicular(z));
        let y = z.cross(x);
        Matrix4::from_cols(
            x.extend(0.0),
            y.extend(0.0),
            z.extend(0.0),
            w.to_vec().extend(1.0),
        )
    }
}

/// `curve`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(generate_deserialize)]
pub enum CurveAny {
    #[holder(use_place_holder)]
    Line(Box<Line>),
    #[holder(use_place_holder)]
    BoundedCurve(Box<BoundedCurveAny>),
    #[holder(use_place_holder)]
    Conic(Box<Conic>),
    #[holder(use_place_holder)]
    Pcurve(Box<Pcurve>),
    #[holder(use_place_holder)]
    SurfaceCurve(Box<SurfaceCurve>),
}

impl TryFrom<&CurveAny> for Curve2D {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(curve: &CurveAny) -> Result<Self, Self::Error> {
        use CurveAny::*;
        Ok(match curve {
            Line(line) => Self::Line(line.as_ref().into()),
            BoundedCurve(b) => b.as_ref().try_into()?,
            Conic(curve) => Self::Conic(curve.as_ref().try_into()?),
            Pcurve(_) => return Err("Pcurves cannot be parsed to 2D curves.".into()),
            SurfaceCurve(_) => return Err("Surface curves cannot be parsed to 2D curves.".into()),
        })
    }
}

impl TryFrom<&CurveAny> for Curve3D {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(curve: &CurveAny) -> Result<Self, Self::Error> {
        use CurveAny::*;
        Ok(match curve {
            Line(line) => Self::Line(line.as_ref().into()),
            BoundedCurve(b) => b.as_ref().try_into()?,
            Conic(curve) => Self::Conic(curve.as_ref().try_into()?),
            Pcurve(c) => Self::PCurve(c.as_ref().try_into()?),
            SurfaceCurve(c) => c.as_ref().try_into()?,
        })
    }
}

/// `line`
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = line)]
#[holder(generate_deserialize)]
pub struct Line {
    pub label: String,
    #[holder(use_place_holder)]
    pub pnt: CartesianPoint,
    #[holder(use_place_holder)]
    pub dir: Vector,
}
impl<'a, P> From<&'a Line> for truck::Line<P>
where
    P: EuclideanSpace + From<&'a CartesianPoint>,
    P::Diff: From<&'a Vector>,
{
    #[inline(always)]
    fn from(line: &'a Line) -> Self {
        let p = P::from(&line.pnt);
        let q = p + P::Diff::from(&line.dir);
        Self(p, q)
    }
}

/// `bounded_curve`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(generate_deserialize)]
pub enum BoundedCurveAny {
    #[holder(use_place_holder)]
    Polyline(Box<Polyline>),
    #[holder(use_place_holder)]
    BSplineCurve(Box<BSplineCurveAny>),
}

impl TryFrom<&BoundedCurveAny> for Curve2D {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(value: &BoundedCurveAny) -> Result<Self, Self::Error> {
        use BoundedCurveAny::*;
        Ok(match value {
            Polyline(x) => Self::Polyline(x.as_ref().into()),
            BSplineCurve(x) => x.as_ref().try_into()?,
        })
    }
}

impl TryFrom<&BoundedCurveAny> for Curve3D {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(value: &BoundedCurveAny) -> Result<Self, Self::Error> {
        use BoundedCurveAny::*;
        Ok(match value {
            Polyline(x) => Self::Polyline(x.as_ref().into()),
            BSplineCurve(x) => x.as_ref().try_into()?,
        })
    }
}

/// `polyline`
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = polyline)]
#[holder(generate_deserialize)]
pub struct Polyline {
    pub label: String,
    #[holder(use_place_holder)]
    pub points: Vec<CartesianPoint>,
}
impl<'a, P: From<&'a CartesianPoint>> From<&'a Polyline> for PolylineCurve<P> {
    #[inline(always)]
    fn from(poly: &'a Polyline) -> Self {
        Self(poly.points.iter().map(|pt| P::from(pt)).collect())
    }
}

/// `b_spline_curve_form`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BSplineCurveForm {
    PolylineForm,
    CircularArc,
    EllipticArc,
    ParabolicArc,
    HyperbolicArc,
    Unspecified,
}

/// `knot_type`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnotType {
    UniformKnots,
    Unspecified,
    QuasiUniformKnots,
    PiecewiseBezierKnots,
}

/// `b_spline_curve_with_knots`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = b_spline_curve_with_knots)]
#[holder(generate_deserialize)]
pub struct BSplineCurveWithKnots {
    pub label: String,
    pub degree: i64,
    #[holder(use_place_holder)]
    pub control_points_list: Vec<CartesianPoint>,
    pub curve_form: BSplineCurveForm,
    pub closed_curve: Logical,
    pub self_intersect: Logical,
    pub knot_multiplicities: Vec<i64>,
    pub knots: Vec<f64>,
    pub knot_spec: KnotType,
}
impl<P: for<'a> From<&'a CartesianPoint>> TryFrom<&BSplineCurveWithKnots> for BSplineCurve<P> {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(curve: &BSplineCurveWithKnots) -> Result<Self, StepConvertingError> {
        let ctx = ToleranceCtx::unscaled_legacy();
        let knots = curve.knots.clone();
        let multi = curve
            .knot_multiplicities
            .iter()
            .map(|n| *n as usize)
            .collect();
        let ctrpts: Vec<P> = curve.control_points_list.iter().map(Into::into).collect();
        let degree = curve.degree as usize;
        let ctrl_count = ctrpts.len();
        let mut kv =
            ValidatedKnotVector::validate(knots, multi, degree, ctrl_count, None)?.into_inner();
        // A STEP exporter may parameterize a perfectly valid curve over a tiny,
        // nonzero knot interval (measured: the six lost core_xy edge curves
        // span ~2e-8..6e-7). `BSplineCurve::try_new` treats any total range
        // under `TOLERANCE` (1e-6, absolute) as zero and refuses it, which
        // would surface here as `EdgeCurveConversionFailed` even though the
        // source curve is well-formed. Normalizing the knot vector to `[0, 1]`
        // is an exact, shape-preserving reparameterization of the same curve,
        // so it is the faithful recovery of the source geometry rather than an
        // approximation. `ValidatedKnotVector::validate` has already proved
        // the active domain is nonzero (it rejects `<= 1e-12`), so `transform`
        // never divides by a zero range here.
        if ctx.is_small_ratio(kv.range_length()) {
            // BG-TOL-001: param
            let range = kv.range_length();
            kv.transform(1.0 / range, -kv[0] / range);
        }
        Ok(Self::try_new(kv, ctrpts)?)
    }
}

/// `bezier_curve`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = bezier_curve)]
#[holder(generate_deserialize)]
pub struct BezierCurve {
    pub label: String,
    pub degree: i64,
    #[holder(use_place_holder)]
    pub control_points_list: Vec<CartesianPoint>,
    pub curve_form: BSplineCurveForm,
    pub closed_curve: Logical,
    pub self_intersect: Logical,
}
impl<P: for<'a> From<&'a CartesianPoint>> TryFrom<&BezierCurve> for BSplineCurve<P> {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(curve: &BezierCurve) -> Result<Self, StepConvertingError> {
        let degree = curve.degree as usize;
        let knots = KnotVec::bezier_knot(degree);
        let ctrpts = curve.control_points_list.iter().map(Into::into).collect();
        Ok(Self::try_new(knots, ctrpts)?)
    }
}

/// `quasi_uniform_curve`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = quasi_uniform_curve)]
#[holder(generate_deserialize)]
pub struct QuasiUniformCurve {
    pub label: String,
    pub degree: i64,
    #[holder(use_place_holder)]
    pub control_points_list: Vec<CartesianPoint>,
    pub curve_form: BSplineCurveForm,
    pub closed_curve: Logical,
    pub self_intersect: Logical,
}
impl<P: for<'a> From<&'a CartesianPoint>> TryFrom<&QuasiUniformCurve> for BSplineCurve<P> {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(curve: &QuasiUniformCurve) -> Result<Self, StepConvertingError> {
        let knots = quasi_uniform_knots(curve.control_points_list.len(), curve.degree as usize)?;
        let ctrpts = curve.control_points_list.iter().map(Into::into).collect();
        Ok(Self::try_new(knots, ctrpts)?)
    }
}

fn quasi_uniform_knots(num_ctrl: usize, degree: usize) -> Result<KnotVec, StepConvertingError> {
    if num_ctrl <= degree {
        // The synthesis path has no source knot list to preserve, so the
        // witness carries the same shape `validate` uses for its
        // length-mismatch arm: empty raw lists and a zeroed active domain.
        let witness = truck_geometry::nurbs::SplineSourceWitness {
            entity_id: None,
            raw_knots: Vec::new(),
            raw_multiplicities: Vec::new(),
            degree,
            control_point_count: num_ctrl,
            expanded_knots: Vec::new(),
            active_domain: (0.0, 0.0),
            first_inversion_index: None,
        };
        return Err(
            truck_geometry::nurbs::SplineConstructionError::ControlPointCountMismatch { witness }
                .into(),
        );
    }
    let division = num_ctrl - degree;
    let mut knots = KnotVec::uniform_knot(degree, division);
    knots.transform(division as f64, 0.0);
    Ok(knots)
}

/// `uniform_curve`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = uniform_curve)]
#[holder(generate_deserialize)]
pub struct UniformCurve {
    pub label: String,
    pub degree: i64,
    #[holder(use_place_holder)]
    pub control_points_list: Vec<CartesianPoint>,
    pub curve_form: BSplineCurveForm,
    pub closed_curve: Logical,
    pub self_intersect: Logical,
}
impl<P: for<'a> From<&'a CartesianPoint>> TryFrom<&UniformCurve> for BSplineCurve<P> {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(curve: &UniformCurve) -> Result<Self, StepConvertingError> {
        let knots = uniform_knots(curve.control_points_list.len(), curve.degree as usize)?;
        let ctrpts = curve.control_points_list.iter().map(Into::into).collect();
        Ok(Self::try_new(knots, ctrpts)?)
    }
}

fn uniform_knots(num_ctrl: usize, degree: usize) -> truck::Result<KnotVec> {
    KnotVec::try_from(
        (0..degree + num_ctrl + 1)
            .map(|i| i as f64 - degree as f64)
            .collect::<Vec<_>>(),
    )
}

/// Entity that does not exist in AP042.
/// Curve before rationalization of [`RationalBSplineCurve`] defined by a complex entity
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(generate_deserialize)]
pub enum NonRationalBSplineCurve {
    #[holder(use_place_holder)]
    BSplineCurveWithKnots(BSplineCurveWithKnots),
    #[holder(use_place_holder)]
    BezierCurve(BezierCurve),
    #[holder(use_place_holder)]
    QuasiUniformCurve(QuasiUniformCurve),
    #[holder(use_place_holder)]
    UniformCurve(UniformCurve),
}

impl<P: for<'a> From<&'a CartesianPoint>> TryFrom<&NonRationalBSplineCurve> for BSplineCurve<P> {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(curve: &NonRationalBSplineCurve) -> Result<Self, StepConvertingError> {
        use NonRationalBSplineCurve::*;
        match curve {
            BSplineCurveWithKnots(x) => x.try_into(),
            BezierCurve(x) => x.try_into(),
            QuasiUniformCurve(x) => x.try_into(),
            UniformCurve(x) => x.try_into(),
        }
    }
}

/// `rational_b_spline_curve` as complex entity
///
/// This struct is an ad hoc implementation that differs from the definition by EXPRESS:
/// in AP042, rationalized curves are defined as complex entities,
/// but here the curves before rationalization are held as internal variables.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = rational_b_spline_curve)]
#[holder(generate_deserialize)]
pub struct RationalBSplineCurve {
    #[holder(use_place_holder)]
    pub non_rational_b_spline_curve: NonRationalBSplineCurve,
    pub weights_data: Vec<f64>,
}
impl<V> TryFrom<&RationalBSplineCurve> for NurbsCurve<V>
where
    V: Homogeneous<Scalar = f64>,
    V::Point: for<'a> From<&'a CartesianPoint>,
{
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(curve: &RationalBSplineCurve) -> Result<Self, StepConvertingError> {
        Ok(Self::try_from_bspline_and_weights(
            BSplineCurve::try_from(&curve.non_rational_b_spline_curve)?,
            curve.weights_data.clone(),
        )?)
    }
}

/// b_spline_curve
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(generate_deserialize)]
pub enum BSplineCurveAny {
    #[holder(use_place_holder)]
    NonRationalBSplineCurve(Box<NonRationalBSplineCurve>),
    #[holder(use_place_holder)]
    RationalBSplineCurve(Box<RationalBSplineCurve>),
}

impl TryFrom<&BSplineCurveAny> for Curve2D {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(value: &BSplineCurveAny) -> Result<Self, Self::Error> {
        use BSplineCurveAny::*;
        Ok(match value {
            NonRationalBSplineCurve(bsp) => Self::BSplineCurve(bsp.as_ref().try_into()?),
            RationalBSplineCurve(bsp) => Self::NurbsCurve(bsp.as_ref().try_into()?),
        })
    }
}

impl TryFrom<&BSplineCurveAny> for Curve3D {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(value: &BSplineCurveAny) -> Result<Self, Self::Error> {
        use BSplineCurveAny::*;
        Ok(match value {
            NonRationalBSplineCurve(bsp) => Self::BSplineCurve(bsp.as_ref().try_into()?),
            RationalBSplineCurve(bsp) => Self::NurbsCurve(bsp.as_ref().try_into()?),
        })
    }
}

/// `conic`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(generate_deserialize)]
pub enum Conic {
    #[holder(use_place_holder)]
    Circle(Circle),
    #[holder(use_place_holder)]
    Ellipse(Ellipse),
    #[holder(use_place_holder)]
    Hyperbola(Hyperbola),
    #[holder(use_place_holder)]
    Parabola(Parabola),
}

impl TryFrom<&Conic> for Conic2D {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(value: &Conic) -> Result<Self, Self::Error> {
        Ok(match value {
            // The source said `circle`. Keeping that is the whole point:
            // see `Conic3D::Circle`.
            Conic::Circle(value) => Conic2D::Circle(value.try_into()?),
            Conic::Ellipse(value) => Conic2D::Ellipse(value.try_into()?),
            Conic::Hyperbola(value) => Conic2D::Hyperbola(value.try_into()?),
            Conic::Parabola(value) => Conic2D::Parabola(value.try_into()?),
        })
    }
}

impl TryFrom<&Conic> for Conic3D {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(value: &Conic) -> Result<Self, Self::Error> {
        Ok(match value {
            Conic::Circle(value) => Conic3D::Circle(value.try_into()?),
            Conic::Ellipse(value) => Conic3D::Ellipse(value.try_into()?),
            Conic::Hyperbola(value) => Conic3D::Hyperbola(value.try_into()?),
            Conic::Parabola(value) => Conic3D::Parabola(value.try_into()?),
        })
    }
}

/// `circle`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = circle)]
#[holder(generate_deserialize)]
pub struct Circle {
    pub label: String,
    #[holder(use_place_holder)]
    pub position: Axis2Placement,
    pub radius: f64,
}

impl TryFrom<&Circle> for step_geometry::Ellipse<Point2, Matrix3> {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(circle: &Circle) -> Result<Self, Self::Error> {
        let transform = Matrix3::try_from(&circle.position)? * Matrix3::from_scale(circle.radius);
        Ok(
            Processor::new(truck::TrimmedCurve::new(UnitCircle::new(), (0.0, 2.0 * PI)))
                .transformed(transform),
        )
    }
}

impl TryFrom<&Circle> for step_geometry::Ellipse<Point3, Matrix4> {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(circle: &Circle) -> Result<Self, Self::Error> {
        let transform = Matrix4::try_from(&circle.position)? * Matrix4::from_scale(circle.radius);
        Ok(
            Processor::new(truck::TrimmedCurve::new(UnitCircle::new(), (0.0, 2.0 * PI)))
                .transformed(transform),
        )
    }
}

/// `ellipse`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = ellipse)]
#[holder(generate_deserialize)]
pub struct Ellipse {
    pub label: String,
    #[holder(use_place_holder)]
    pub position: Axis2Placement,
    pub semi_axis_1: f64,
    pub semi_axis_2: f64,
}

impl TryFrom<&Ellipse> for step_geometry::Ellipse<Point2, Matrix3> {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(ellipse: &Ellipse) -> Result<Self, Self::Error> {
        let (r0, r1) = (ellipse.semi_axis_1, ellipse.semi_axis_2);
        let transform =
            Matrix3::try_from(&ellipse.position)? * Matrix3::from_nonuniform_scale(r0, r1);
        Ok(
            Processor::new(truck::TrimmedCurve::new(UnitCircle::new(), (0.0, 2.0 * PI)))
                .transformed(transform),
        )
    }
}

impl TryFrom<&Ellipse> for step_geometry::Ellipse<Point3, Matrix4> {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(ellipse: &Ellipse) -> Result<Self, Self::Error> {
        let (r0, r1) = (ellipse.semi_axis_1, ellipse.semi_axis_2);
        let transform = Matrix4::try_from(&ellipse.position)?
            * Matrix4::from_nonuniform_scale(r0, r1, f64::min(r0, r1));
        Ok(
            Processor::new(truck::TrimmedCurve::new(UnitCircle::new(), (0.0, 2.0 * PI)))
                .transformed(transform),
        )
    }
}

/// `hyperbola`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = hyperbola)]
#[holder(generate_deserialize)]
pub struct Hyperbola {
    pub label: String,
    #[holder(use_place_holder)]
    pub position: Axis2Placement,
    pub semi_axis: f64,
    pub semi_imag_axis: f64,
}

impl TryFrom<&Hyperbola> for step_geometry::Hyperbola<Point2, Matrix3> {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(hyperbola: &Hyperbola) -> Result<Self, Self::Error> {
        let (r0, r1) = (hyperbola.semi_axis, hyperbola.semi_imag_axis);
        let transform =
            Matrix3::try_from(&hyperbola.position)? * Matrix3::from_nonuniform_scale(r0, r1);
        Ok(
            Processor::new(truck::TrimmedCurve::new(UnitHyperbola::new(), (-1.0, 1.0)))
                .transformed(transform),
        )
    }
}

impl TryFrom<&Hyperbola> for step_geometry::Hyperbola<Point3, Matrix4> {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(hyperbola: &Hyperbola) -> Result<Self, Self::Error> {
        let (r0, r1) = (hyperbola.semi_axis, hyperbola.semi_imag_axis);
        let transform = Matrix4::try_from(&hyperbola.position)?
            * Matrix4::from_nonuniform_scale(r0, r1, f64::min(r0, r1));
        Ok(
            Processor::new(truck::TrimmedCurve::new(UnitHyperbola::new(), (-1.0, 1.0)))
                .transformed(transform),
        )
    }
}

/// `parabola`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = parabola)]
#[holder(generate_deserialize)]
pub struct Parabola {
    pub label: String,
    #[holder(use_place_holder)]
    pub position: Axis2Placement,
    pub focal_dist: f64,
}

impl TryFrom<&Parabola> for step_geometry::Parabola<Point2, Matrix3> {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(parabola: &Parabola) -> Result<Self, Self::Error> {
        let transform =
            Matrix3::try_from(&parabola.position)? * Matrix3::from_scale(parabola.focal_dist);
        Ok(
            Processor::new(truck::TrimmedCurve::new(UnitParabola::new(), (-1.0, 1.0)))
                .transformed(transform),
        )
    }
}

impl TryFrom<&Parabola> for step_geometry::Parabola<Point3, Matrix4> {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(parabola: &Parabola) -> Result<Self, Self::Error> {
        let transform =
            Matrix4::try_from(&parabola.position)? * Matrix4::from_scale(parabola.focal_dist);
        Ok(
            Processor::new(truck::TrimmedCurve::new(UnitParabola::new(), (-1.0, 1.0)))
                .transformed(transform),
        )
    }
}

/// `definitional_representation`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = definitional_representation)]
#[holder(generate_deserialize)]
pub struct DefinitionalRepresentation {
    label: String,
    #[holder(use_place_holder)]
    representation_item: Vec<CurveAny>,
    #[holder(use_place_holder)]
    context_of_items: Dummy,
}

/// `pcurve`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = pcurve)]
#[holder(generate_deserialize)]
pub struct Pcurve {
    label: String,
    #[holder(use_place_holder)]
    basis_surface: SurfaceAny,
    #[holder(use_place_holder)]
    reference_to_curve: DefinitionalRepresentation,
}

impl TryFrom<&Pcurve> for PCurve {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(value: &Pcurve) -> Result<Self, Self::Error> {
        let surface: Surface = (&value.basis_surface).try_into()?;
        let curve: Curve2D = value
            .reference_to_curve
            .representation_item
            .first()
            .ok_or("no representation item")?
            .try_into()?;
        Ok(step_geometry::PCurve::new(
            Box::new(curve),
            Box::new(surface),
        ))
    }
}

/// `pcurve_or_surface`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(generate_deserialize)]
pub enum PcurveOrSurface {
    #[holder(use_place_holder)]
    Pcurve(Box<Pcurve>),
    #[holder(use_place_holder)]
    Surface(Box<SurfaceAny>),
}

/// `preferred_surface_representation`
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum PreferredSurfaceCurveRepresentation {
    Curve3D,
    PcurveS1,
    PcurveS2,
}

#[test]
fn deserialize_pscr() {
    let (_, p) = ruststep::parser::exchange::parameter(".PCURVE_S1.").unwrap();
    let x = PreferredSurfaceCurveRepresentation::deserialize(&p).unwrap();
    assert!(matches!(x, PreferredSurfaceCurveRepresentation::PcurveS1));
    let (_, p) = ruststep::parser::exchange::parameter(".PCURVE_S2.").unwrap();
    let x = PreferredSurfaceCurveRepresentation::deserialize(&p).unwrap();
    assert!(matches!(x, PreferredSurfaceCurveRepresentation::PcurveS2));
}

/// `surface_curve`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = surface_curve)]
#[holder(generate_deserialize)]
pub struct SurfaceCurve {
    label: String,
    #[holder(use_place_holder)]
    curve_3d: CurveAny,
    #[holder(use_place_holder)]
    associated_geometry: Vec<PcurveOrSurface>,
    master_representation: PreferredSurfaceCurveRepresentation,
}

impl TryFrom<&SurfaceCurve> for Curve3D {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(value: &SurfaceCurve) -> Result<Self, Self::Error> {
        use PreferredSurfaceCurveRepresentation as PSCR;
        match &value.master_representation {
            PSCR::Curve3D => Ok((&value.curve_3d).try_into()?),
            PSCR::PcurveS1 => {
                if let Some(PcurveOrSurface::Pcurve(x)) = value.associated_geometry.first() {
                    Ok(Self::PCurve(x.as_ref().try_into()?))
                } else {
                    Err("The 0-indexed associated geometry is nothing or not PCURVE.".into())
                }
            }
            PSCR::PcurveS2 => {
                if let Some(PcurveOrSurface::Pcurve(x)) = value.associated_geometry.get(1) {
                    Ok(Self::PCurve(x.as_ref().try_into()?))
                } else {
                    Err("The 1-indexed associated geometry is nothing or not PCURVE.".into())
                }
            }
        }
    }
}

/// `surface`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(generate_deserialize)]
pub enum SurfaceAny {
    #[holder(use_place_holder)]
    ElementarySurface(Box<ElementarySurfaceAny>),
    #[holder(use_place_holder)]
    BSplineSurface(Box<BSplineSurfaceAny>),
    #[holder(use_place_holder)]
    SweptSurface(Box<SweptSurfaceAny>),
    #[holder(use_place_holder)]
    OffsetSurface(Box<OffsetSurface>),
}

impl TryFrom<&SurfaceAny> for Surface {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(x: &SurfaceAny) -> Result<Self, Self::Error> {
        use SurfaceAny::*;
        Ok(match x {
            ElementarySurface(x) => Self::ElementarySurface(x.as_ref().try_into()?),
            BSplineSurface(x) => x.as_ref().try_into()?,
            SweptSurface(x) => Self::SweptCurve(x.as_ref().try_into()?),
            OffsetSurface(x) => Self::OffsetSurface(x.as_ref().try_into()?),
        })
    }
}

/// `elementary_surface`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(generate_deserialize)]
pub enum ElementarySurfaceAny {
    #[holder(use_place_holder)]
    Plane(Box<Plane>),
    #[holder(use_place_holder)]
    SphericalSurface(Box<SphericalSurface>),
    #[holder(use_place_holder)]
    CylindricalSurface(Box<CylindricalSurface>),
    #[holder(use_place_holder)]
    ToroidalSurface(Box<ToroidalSurface>),
    #[holder(use_place_holder)]
    DegenerateToroidalSurface(Box<DegenerateToroidalSurface>),
    #[holder(use_place_holder)]
    ConicalSurface(Box<ConicalSurface>),
}

impl TryFrom<&ElementarySurfaceAny> for ElementarySurface {
    type Error = StepConvertingError;
    fn try_from(value: &ElementarySurfaceAny) -> Result<Self, Self::Error> {
        use ElementarySurfaceAny::*;
        Ok(match value {
            Plane(x) => Self::Plane(x.as_ref().into()),
            SphericalSurface(x) => Self::Sphere(x.as_ref().into()),
            CylindricalSurface(x) => Self::CylindricalSurface(x.as_ref().into()),
            ToroidalSurface(x) => Self::ToroidalSurface(x.as_ref().into()),
            DegenerateToroidalSurface(x) => Self::DegenerateToroidalSurface(x.as_ref().try_into()?),
            ConicalSurface(x) => Self::ConicalSurface(x.as_ref().into()),
        })
    }
}

/// `plane`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = plane)]
#[holder(generate_deserialize)]
pub struct Plane {
    label: String,
    #[holder(use_place_holder)]
    position: Axis2Placement3d,
}

impl From<&Plane> for truck::Plane {
    #[inline(always)]
    fn from(plane: &Plane) -> Self {
        let mat = Matrix4::from(&plane.position);
        let o = Point3::from_homogeneous(mat[3]);
        let p = o + mat[0].truncate();
        let q = o + mat[1].truncate();
        Self::new(o, p, q)
    }
}

/// `offset_surface`
///
/// A surface at a constant distance from a basis surface along that basis'
/// own normal. CAD systems emit these constantly for shelled and thickened
/// parts: a quarter of the real files sampled from the ABC dataset contain
/// them, though the curated NIST corpus contains none.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = offset_surface)]
#[holder(generate_deserialize)]
pub struct OffsetSurface {
    label: String,
    #[holder(use_place_holder)]
    basis_surface: SurfaceAny,
    distance: f64,
    self_intersect: Logical,
}

impl TryFrom<&OffsetSurface> for step_geometry::StepOffsetSurface {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(x: &OffsetSurface) -> Result<Self, Self::Error> {
        let basis: Surface = (&x.basis_surface).try_into()?;
        Ok(Self::new(basis, x.distance))
    }
}

/// `spherical_surface`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = spherical_surface)]
#[holder(generate_deserialize)]
pub struct SphericalSurface {
    label: String,
    #[holder(use_place_holder)]
    position: Axis2Placement3d,
    radius: f64,
}

impl From<&SphericalSurface> for step_geometry::SphericalSurface {
    #[inline(always)]
    fn from(ss: &SphericalSurface) -> Self {
        let mat = Matrix4::from(&ss.position);
        let sphere = Sphere(truck::Sphere::new(Point3::origin(), ss.radius));
        Processor::new(sphere).transformed(mat)
    }
}

/// `cylindrical_surface`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = cylindrical_surface)]
#[holder(generate_deserialize)]
pub struct CylindricalSurface {
    label: String,
    #[holder(use_place_holder)]
    position: Axis2Placement3d,
    radius: f64,
}

impl From<&CylindricalSurface> for step_geometry::CylindricalSurface {
    #[inline(always)]
    fn from(cs: &CylindricalSurface) -> Self {
        let mat = Matrix4::from(&cs.position);
        let x = mat[0].truncate();
        let z = mat[2].truncate();
        let center = Point3::from_homogeneous(mat[3]);
        let radius = cs.radius;
        let p = center + x * radius;
        let mut res = Processor::new(RevolutedCurve::by_revolution(Line(p, p + z), center, z));
        res.invert();
        res
    }
}

/// `toroidal_surface`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = toroidal_surface)]
#[holder(generate_deserialize)]
pub struct ToroidalSurface {
    label: String,
    #[holder(use_place_holder)]
    position: Axis2Placement3d,
    major_radius: f64,
    minor_radius: f64,
}

impl From<&ToroidalSurface> for step_geometry::ToroidalSurface {
    #[inline(always)]
    fn from(
        ToroidalSurface {
            position,
            major_radius,
            minor_radius,
            ..
        }: &ToroidalSurface,
    ) -> Self {
        let mat = Matrix4::from(position);
        let torus = Torus::new(Point3::origin(), *major_radius, *minor_radius);
        Processor::new(torus).transformed(mat)
    }
}

/// `degenerate_toroidal_surface`
///
/// An AP242 subtype of `toroidal_surface` carrying the same carrier geometry
/// plus a `select_outer` sheet flag. The EXPRESS WHERE clause fixes
/// `major_radius < minor_radius`, so the carrier is a self-intersecting
/// (spindle) torus and the face must name which of the two sheets of the
/// parametrisation it lies on. The conversion preserves that source-defined
/// sheet as a restricted parameter domain on the existing torus carrier; it is
/// not a full-`[0, 2π]` torus.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = degenerate_toroidal_surface)]
#[holder(generate_deserialize)]
pub struct DegenerateToroidalSurface {
    label: String,
    #[holder(use_place_holder)]
    position: Axis2Placement3d,
    major_radius: f64,
    minor_radius: f64,
    select_outer: bool,
}

impl TryFrom<&DegenerateToroidalSurface> for step_geometry::DegenerateToroidalSurface {
    type Error = StepConvertingError;
    fn try_from(
        DegenerateToroidalSurface {
            position,
            major_radius,
            minor_radius,
            select_outer,
            ..
        }: &DegenerateToroidalSurface,
    ) -> Result<Self, Self::Error> {
        let carrier =
            step_geometry::DegenerateTorus::new(*major_radius, *minor_radius, *select_outer)
                .ok_or_else(|| {
                    "degenerate_toroidal_surface: radii must be positive and finite with \
             major_radius < minor_radius (EXPRESS WHERE)"
                        .to_string()
                })?;
        let mat = Matrix4::from(position);
        Ok(Processor::new(carrier).transformed(mat))
    }
}

/// `conical_surface`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = conical_surface)]
#[holder(generate_deserialize)]
pub struct ConicalSurface {
    label: String,
    #[holder(use_place_holder)]
    position: Axis2Placement3d,
    radius: f64,
    semi_angle: f64,
}

impl From<&ConicalSurface> for step_geometry::ConicalSurface {
    fn from(
        ConicalSurface {
            position,
            radius,
            semi_angle,
            ..
        }: &ConicalSurface,
    ) -> Self {
        let mat = Matrix4::from(position);
        // EXPERIMENT (TRUCK_CONE_APEX_RANGE): span the generatrix from the apex
        // to twice the reference radius, instead of one unit outward from the
        // reference circle.
        //
        // `Line::parameter_range()` is `[0,1]` unconditionally, so the declared
        // domain of the revolved surface is whatever one unit of the generatrix
        // covers. With the direction below that is axial z in [0,1] starting at
        // the reference circle -- a slab that excludes the apex at
        // u* = -R/tan(theta) and, for any cone taller than one unit, most of the
        // face as well. Boundary stitching closes an open piece against the edge
        // of that domain, and when the piece lies on the edge the enclosed area
        // is zero and nothing meshes.
        let tan = f64::tan(*semi_angle);
        let p = Point3::new(*radius, 0.0, 0.0);
        let v = Vector3::new(tan, 0.0, 1.0);
        let rev =
            RevolutedCurve::by_revolution(Line(p, p + v), Point3::origin(), Vector3::unit_z());
        let mut processor = Processor::new(rev);
        processor.transform_by(mat);
        processor.invert();
        processor
    }
}

/// `b_spline_surface`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(generate_deserialize)]
pub enum BSplineSurfaceAny {
    #[holder(use_place_holder)]
    NonRationalBSplineSurface(NonRationalBSplineSurface),
    #[holder(use_place_holder)]
    RationalBSplineSurface(RationalBSplineSurface),
}

impl TryFrom<&BSplineSurfaceAny> for Surface {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(value: &BSplineSurfaceAny) -> Result<Self, Self::Error> {
        use BSplineSurfaceAny::*;
        Ok(match value {
            NonRationalBSplineSurface(bsp) => Surface::BSplineSurface(bsp.try_into()?),
            RationalBSplineSurface(bsp) => Surface::NurbsSurface(bsp.try_into()?),
        })
    }
}

/// `b_spline_surface_form`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BSplineSurfaceForm {
    PlaneSurf,
    CylindricalSurf,
    ConicalSurf,
    SphericalSurf,
    ToroidalSurf,
    SurfOfRevolution,
    RuledSurf,
    GeneralisedCone,
    QuadricSurf,
    SurfOfLinearExtrusion,
    Unspecified,
}

/// `b_spline_surface_with_knots`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = b_spline_surface_with_knots)]
#[holder(generate_deserialize)]
pub struct BSplineSurfaceWithKnots {
    label: String,
    u_degree: i64,
    v_degree: i64,
    #[holder(use_place_holder)]
    control_points_list: Vec<Vec<CartesianPoint>>,
    surface_form: BSplineSurfaceForm,
    u_closed: Logical,
    v_closed: Logical,
    self_intersect: Logical,
    u_multiplicities: Vec<i64>,
    v_multiplicities: Vec<i64>,
    u_knots: Vec<f64>,
    v_knots: Vec<f64>,
    knot_spec: KnotType,
}

impl BSplineSurfaceWithKnots {
    /// The source-declared closure of the `u` axis.
    ///
    /// Only an explicit `.T.` is closure. `Unknown` (`.U.`) and `.F.` are
    /// both "not declared closed"; the STEP declaration is the authority and
    /// nothing downstream may infer closure from the geometry.
    pub fn u_closed(&self) -> bool {
        matches!(self.u_closed, Logical::True)
    }

    /// The source-declared closure of the `v` axis.
    ///
    /// See [`Self::u_closed`] for the semantics.
    pub fn v_closed(&self) -> bool {
        matches!(self.v_closed, Logical::True)
    }
}

impl TryFrom<&BSplineSurfaceWithKnots> for BSplineSurface<Point3> {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(surface: &BSplineSurfaceWithKnots) -> Result<Self, StepConvertingError> {
        let uknots = surface.u_knots.to_vec();
        let umulti: Vec<usize> = surface
            .u_multiplicities
            .iter()
            .map(|n| *n as usize)
            .collect();
        let vknots = surface.v_knots.to_vec();
        let vmulti: Vec<usize> = surface
            .v_multiplicities
            .iter()
            .map(|n| *n as usize)
            .collect();
        let ctrls: Vec<Vec<Point3>> = surface
            .control_points_list
            .iter()
            .map(|vec| vec.iter().map(Point3::from).collect())
            .collect();

        let u_degree = surface.u_degree as usize;
        let u_ctrl_count = ctrls.len();
        let ctx = ToleranceCtx::unscaled_legacy();
        let mut u_kv = ValidatedKnotVector::validate(
            uknots.clone(),
            umulti.clone(),
            u_degree,
            u_ctrl_count,
            None,
        )?
        .into_inner();
        // A STEP exporter may parameterize a perfectly valid surface over a
        // tiny, nonzero knot interval on the u axis. `BSplineSurface::try_new`
        // treats any total u-axis range under `TOLERANCE` (absolute) as zero
        // and refuses it, even though the source surface is well-formed.
        // Normalizing the u knot vector to `[0, 1]` is an exact,
        // shape-preserving reparameterization of the same surface, so it is
        // the faithful recovery of the source geometry rather than an
        // approximation. `ValidatedKnotVector::validate` has already proved
        // the active domain exceeds `1e-12`; // H-3: dimensionless parameter-space bound, not a model length
        // the active domain is a subinterval of the total range, so `range` is
        // strictly positive and `transform` never divides by zero here.
        if ctx.is_small_ratio(u_kv.range_length()) {
            // BG-TOL-001: param
            let range = u_kv.range_length();
            u_kv.transform(1.0 / range, -u_kv[0] / range);
        }

        let v_degree = surface.v_degree as usize;
        let v_ctrl_count = if ctrls.is_empty() { 0 } else { ctrls[0].len() };
        let mut v_kv = ValidatedKnotVector::validate(
            vknots.clone(),
            vmulti.clone(),
            v_degree,
            v_ctrl_count,
            None,
        )?
        .into_inner();
        // A STEP exporter may parameterize a perfectly valid surface over a
        // tiny, nonzero knot interval on the v axis. `BSplineSurface::try_new`
        // treats any total v-axis range under `TOLERANCE` (absolute) as zero
        // and refuses it, even though the source surface is well-formed.
        // Normalizing the v knot vector to `[0, 1]` is an exact,
        // shape-preserving reparameterization of the same surface, so it is
        // the faithful recovery of the source geometry rather than an
        // approximation. `ValidatedKnotVector::validate` has already proved
        // the active domain exceeds `1e-12`; // H-3: dimensionless parameter-space bound, not a model length
        // the active domain is a subinterval of the total range, so `range` is
        // strictly positive and `transform` never divides by zero here.
        if ctx.is_small_ratio(v_kv.range_length()) {
            // BG-TOL-001: param
            let range = v_kv.range_length();
            v_kv.transform(1.0 / range, -v_kv[0] / range);
        }

        Ok(Self::try_new((u_kv, v_kv), ctrls)?)
    }
}

/// `uniform_surface`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = uniform_surface)]
#[holder(generate_deserialize)]
pub struct UniformSurface {
    label: String,
    u_degree: i64,
    v_degree: i64,
    #[holder(use_place_holder)]
    control_points_list: Vec<Vec<CartesianPoint>>,
    surface_form: BSplineSurfaceForm,
    u_closed: Logical,
    v_closed: Logical,
    self_intersect: Logical,
}

impl UniformSurface {
    /// The source-declared closure of the `u` axis.
    pub fn u_closed(&self) -> bool {
        matches!(self.u_closed, Logical::True)
    }

    /// The source-declared closure of the `v` axis.
    pub fn v_closed(&self) -> bool {
        matches!(self.v_closed, Logical::True)
    }
}

impl TryFrom<&UniformSurface> for BSplineSurface<Point3> {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(surface: &UniformSurface) -> Result<Self, StepConvertingError> {
        let uknots = uniform_knots(surface.control_points_list.len(), surface.u_degree as usize)?;
        let first = surface
            .control_points_list
            .first()
            .ok_or("control points list is empty.")?;
        let vknots = uniform_knots(first.len(), surface.v_degree as usize)?;
        let ctrls = surface
            .control_points_list
            .iter()
            .map(|vec| vec.iter().map(Point3::from).collect())
            .collect();
        Ok(Self::try_new((uknots, vknots), ctrls)?)
    }
}

/// `quasi_uniform_surface`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = quasi_uniform_surface)]
#[holder(generate_deserialize)]
pub struct QuasiUniformSurface {
    label: String,
    u_degree: i64,
    v_degree: i64,
    #[holder(use_place_holder)]
    control_points_list: Vec<Vec<CartesianPoint>>,
    surface_form: BSplineSurfaceForm,
    u_closed: Logical,
    v_closed: Logical,
    self_intersect: Logical,
}

impl QuasiUniformSurface {
    /// The source-declared closure of the `u` axis.
    pub fn u_closed(&self) -> bool {
        matches!(self.u_closed, Logical::True)
    }

    /// The source-declared closure of the `v` axis.
    pub fn v_closed(&self) -> bool {
        matches!(self.v_closed, Logical::True)
    }
}

impl TryFrom<&QuasiUniformSurface> for BSplineSurface<Point3> {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(surface: &QuasiUniformSurface) -> Result<Self, StepConvertingError> {
        let uknots =
            quasi_uniform_knots(surface.control_points_list.len(), surface.u_degree as usize)?;
        let first = surface
            .control_points_list
            .first()
            .ok_or("control points list is empty.")?;
        let vknots = quasi_uniform_knots(first.len(), surface.v_degree as usize)?;
        let ctrls = surface
            .control_points_list
            .iter()
            .map(|vec| vec.iter().map(Point3::from).collect())
            .collect();
        Ok(Self::try_new((uknots, vknots), ctrls)?)
    }
}

/// `bezier_surface`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = bezier_surface)]
#[holder(generate_deserialize)]
pub struct BezierSurface {
    label: String,
    u_degree: i64,
    v_degree: i64,
    #[holder(use_place_holder)]
    control_points_list: Vec<Vec<CartesianPoint>>,
    surface_form: BSplineSurfaceForm,
    u_closed: Logical,
    v_closed: Logical,
    self_intersect: Logical,
}

impl BezierSurface {
    /// The source-declared closure of the `u` axis.
    pub fn u_closed(&self) -> bool {
        matches!(self.u_closed, Logical::True)
    }

    /// The source-declared closure of the `v` axis.
    pub fn v_closed(&self) -> bool {
        matches!(self.v_closed, Logical::True)
    }
}

impl From<&BezierSurface> for BSplineSurface<Point3> {
    #[inline(always)]
    fn from(value: &BezierSurface) -> Self {
        let uknots = KnotVec::bezier_knot(value.u_degree as usize);
        let vknots = KnotVec::bezier_knot(value.v_degree as usize);
        let ctrls = value
            .control_points_list
            .iter()
            .map(|vec| vec.iter().map(Point3::from).collect())
            .collect();
        Self::new((uknots, vknots), ctrls)
    }
}

/// Entity that does not exist in AP042.
/// Surface before rationalization of [`RationalBSplineSurface`] defined by a complex entity
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(generate_deserialize)]
pub enum NonRationalBSplineSurface {
    #[holder(use_place_holder)]
    BSplineSurfaceWithKnots(Box<BSplineSurfaceWithKnots>),
    #[holder(use_place_holder)]
    UniformSurface(Box<UniformSurface>),
    #[holder(use_place_holder)]
    QuasiUniformSurface(Box<QuasiUniformSurface>),
    #[holder(use_place_holder)]
    BezierSurface(Box<BezierSurface>),
}

impl TryFrom<&NonRationalBSplineSurface> for BSplineSurface<Point3> {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(value: &NonRationalBSplineSurface) -> Result<Self, Self::Error> {
        use NonRationalBSplineSurface::*;
        match value {
            BSplineSurfaceWithKnots(x) => x.as_ref().try_into(),
            UniformSurface(x) => x.as_ref().try_into(),
            QuasiUniformSurface(x) => x.as_ref().try_into(),
            BezierSurface(x) => Ok(x.as_ref().into()),
        }
    }
}

impl NonRationalBSplineSurface {
    /// The source-declared closure of the `u` axis, forwarded from whichever
    /// concrete form the source entity took.
    pub fn u_closed(&self) -> bool {
        use NonRationalBSplineSurface::*;
        match self {
            BSplineSurfaceWithKnots(x) => x.u_closed(),
            UniformSurface(x) => x.u_closed(),
            QuasiUniformSurface(x) => x.u_closed(),
            BezierSurface(x) => x.u_closed(),
        }
    }

    /// The source-declared closure of the `v` axis, forwarded from whichever
    /// concrete form the source entity took.
    pub fn v_closed(&self) -> bool {
        use NonRationalBSplineSurface::*;
        match self {
            BSplineSurfaceWithKnots(x) => x.v_closed(),
            UniformSurface(x) => x.v_closed(),
            QuasiUniformSurface(x) => x.v_closed(),
            BezierSurface(x) => x.v_closed(),
        }
    }
}

/// `rational_b_spline_surface` as complex entity
///
/// This struct is an ad hoc implementation that differs from the definition by EXPRESS:
/// in AP042, rationalized curves are defined as complex entities,
/// but here the surfaces before rationalization are held as internal variables.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = rational_b_spline_surface)]
#[holder(generate_deserialize)]
pub struct RationalBSplineSurface {
    #[holder(use_place_holder)]
    non_rational_b_spline_surface: NonRationalBSplineSurface,
    weights_data: Vec<Vec<f64>>,
}

impl TryFrom<&RationalBSplineSurface> for NurbsSurface<Vector4> {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(
        RationalBSplineSurface {
            non_rational_b_spline_surface,
            weights_data,
        }: &RationalBSplineSurface,
    ) -> Result<Self, Self::Error> {
        let surface: BSplineSurface<Point3> = non_rational_b_spline_surface.try_into()?;
        Ok(Self::try_from_bspline_and_weights(
            surface,
            weights_data.clone(),
        )?)
    }
}

impl RationalBSplineSurface {
    /// The source-declared closure of the `u` axis, forwarded from the wrapped
    /// non-rational form that actually carries the declaration.
    pub fn u_closed(&self) -> bool {
        self.non_rational_b_spline_surface.u_closed()
    }

    /// The source-declared closure of the `v` axis, forwarded from the wrapped
    /// non-rational form that actually carries the declaration.
    pub fn v_closed(&self) -> bool {
        self.non_rational_b_spline_surface.v_closed()
    }
}

/// `swept_surface`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(generate_deserialize)]
pub enum SweptSurfaceAny {
    #[holder(use_place_holder)]
    SurfaceOfLinearExtrusion(Box<SurfaceOfLinearExtrusion>),
    #[holder(use_place_holder)]
    SurfaceOfRevolution(Box<SurfaceOfRevolution>),
}

impl TryFrom<&SweptSurfaceAny> for SweptCurve {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(value: &SweptSurfaceAny) -> Result<Self, Self::Error> {
        use SweptSurfaceAny::*;
        Ok(match value {
            SurfaceOfLinearExtrusion(x) => SweptCurve::ExtrudedCurve(x.as_ref().try_into()?),
            SurfaceOfRevolution(x) => SweptCurve::RevolutedCurve(x.as_ref().try_into()?),
        })
    }
}

/// `surface_of_linear_extrusion`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = surface_of_linear_extrusion)]
#[holder(generate_deserialize)]
pub struct SurfaceOfLinearExtrusion {
    label: String,
    #[holder(use_place_holder)]
    swept_curve: CurveAny,
    #[holder(use_place_holder)]
    extrusion_axis: Vector,
}

impl TryFrom<&SurfaceOfLinearExtrusion> for StepExtrudedCurve {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(sr: &SurfaceOfLinearExtrusion) -> Result<Self, Self::Error> {
        let curve = Curve3D::try_from(&sr.swept_curve)?;
        let vector = Vector3::from(&sr.extrusion_axis);
        Ok(ExtrudedCurve::by_extrusion(curve, vector))
    }
}

/// `surface_of_revolution`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = surface_of_revolution)]
#[holder(generate_deserialize)]
pub struct SurfaceOfRevolution {
    label: String,
    #[holder(use_place_holder)]
    swept_curve: CurveAny,
    #[holder(use_place_holder)]
    axis_position: Axis1Placement,
}

impl TryFrom<&SurfaceOfRevolution> for StepRevolutedCurve {
    type Error = StepConvertingError;
    #[inline(always)]
    fn try_from(sr: &SurfaceOfRevolution) -> Result<Self, Self::Error> {
        let curve = Curve3D::try_from(&sr.swept_curve)?;
        let origin = Point3::from(&sr.axis_position.location);
        let axis = sr.axis_position.direction().normalize();
        let mut rev = Processor::new(RevolutedCurve::by_revolution(curve, origin, axis));
        rev.invert();
        Ok(rev)
    }
}

/// `vertex_point`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = vertex_point)]
#[holder(generate_deserialize)]
pub struct VertexPoint {
    pub label: String,
    #[holder(use_place_holder)]
    pub vertex_geometry: CartesianPoint,
}

/// `edge`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(generate_deserialize)]
pub enum EdgeAny {
    #[holder(use_place_holder)]
    EdgeCurve(EdgeCurve),
    #[holder(use_place_holder)]
    OrientedEdge(OrientedEdge),
}

/// `edge_curve`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = edge_curve)]
#[holder(generate_deserialize)]
pub struct EdgeCurve {
    pub label: String,
    #[holder(use_place_holder)]
    pub edge_start: VertexPoint,
    #[holder(use_place_holder)]
    pub edge_end: VertexPoint,
    #[holder(use_place_holder)]
    pub edge_geometry: CurveAny,
    pub same_sense: bool,
}

impl EdgeCurve {
    pub fn parse_curve2d(&self) -> Result<Curve2D, StepConvertingError> {
        let p = Point2::from(&self.edge_start.vertex_geometry);
        let q = Point2::from(&self.edge_end.vertex_geometry);
        let (p, q) = match self.same_sense {
            true => (p, q),
            false => (q, p),
        };
        Self::sub_parse_2d(&self.edge_geometry, p, q, self.same_sense)
    }
    fn sub_parse_2d(
        curve: &CurveAny,
        p: Point2,
        q: Point2,
        same_sense: bool,
    ) -> Result<Curve2D, StepConvertingError> {
        let ctx = ToleranceCtx::unscaled_legacy();
        let mut curve = match curve {
            CurveAny::Line(line) => {
                let line = truck::Line::<Point2>::from(line.as_ref());
                let p = line.projection(p);
                let q = line.projection(q);
                Curve2D::Line(Line(p, q))
            }
            CurveAny::BoundedCurve(b) => b.as_ref().try_into()?,
            CurveAny::Conic(curve) => match curve.as_ref() {
                Conic::Circle(circle) => {
                    let mat =
                        Matrix3::try_from(&circle.position)? * Matrix3::from_scale(circle.radius);
                    let inv_mat = mat
                        .invert()
                        .ok_or_else(|| "Failed to convert Circle".to_string())?;
                    let (p, q) = (inv_mat.transform_point(p), inv_mat.transform_point(q));
                    let (u, mut v) = (
                        UnitCircle::<Point2>::new()
                            .search_nearest_parameter(p, None, 0)
                            .ok_or_else(|| "the point is not on circle".to_string())?,
                        UnitCircle::<Point2>::new()
                            .search_nearest_parameter(q, None, 0)
                            .ok_or_else(|| "the point is not on circle".to_string())?,
                    );
                    if v < u - ctx.ratio_margin() {
                        // BG-TOL-001: param
                        v += 2.0 * PI;
                    }
                    let circle = TrimmedCurve::new(UnitCircle::<Point2>::new(), (u, v));
                    let mut ellipse = Processor::new(circle);
                    ellipse.transform_by(mat);
                    Curve2D::Conic(Conic2D::Ellipse(ellipse))
                }
                Conic::Ellipse(ellipse) => {
                    let mat = Matrix3::try_from(&ellipse.position)?
                        * Matrix3::from_nonuniform_scale(ellipse.semi_axis_1, ellipse.semi_axis_2);
                    let inv_mat = mat
                        .invert()
                        .ok_or_else(|| "Failed to convert Circle".to_string())?;
                    let (p, q) = (inv_mat.transform_point(p), inv_mat.transform_point(q));
                    let (u, mut v) = (
                        UnitCircle::<Point2>::new()
                            .search_nearest_parameter(p, None, 0)
                            .ok_or_else(|| "the point is not on circle".to_string())?,
                        UnitCircle::<Point2>::new()
                            .search_nearest_parameter(q, None, 0)
                            .ok_or_else(|| "the point is not on circle".to_string())?,
                    );
                    if v < u - ctx.ratio_margin() {
                        // BG-TOL-001: param
                        v += 2.0 * PI;
                    }
                    let circle = TrimmedCurve::new(UnitCircle::<Point2>::new(), (u, v));
                    let mut ellipse = Processor::new(circle);
                    ellipse.transform_by(mat);
                    Curve2D::Conic(Conic2D::Ellipse(ellipse))
                }
                Conic::Hyperbola(hyperbola) => {
                    let mat = Matrix3::try_from(&hyperbola.position)?
                        * Matrix3::from_nonuniform_scale(
                            hyperbola.semi_axis,
                            hyperbola.semi_imag_axis,
                        );
                    let inv_mat = mat
                        .invert()
                        .ok_or_else(|| "Failed to convert Hyperbola".to_string())?;
                    let (p, q) = (inv_mat.transform_point(p), inv_mat.transform_point(q));
                    let (u, v) = (
                        UnitHyperbola::<Point2>::new()
                            .search_nearest_parameter(p, None, 0)
                            .ok_or_else(|| "the point is not on hyperbola".to_string())?,
                        UnitHyperbola::<Point2>::new()
                            .search_nearest_parameter(q, None, 0)
                            .ok_or_else(|| "the point is not on hyparbola".to_string())?,
                    );
                    let unit = TrimmedCurve::new(UnitHyperbola::<Point2>::new(), (u, v));
                    let mut hyperbola = Processor::new(unit);
                    hyperbola.transform_by(mat);
                    Curve2D::Conic(Conic2D::Hyperbola(hyperbola))
                }
                Conic::Parabola(parabola) => {
                    let mat = Matrix3::try_from(&parabola.position)?
                        * Matrix3::from_scale(parabola.focal_dist);
                    let inv_mat = mat
                        .invert()
                        .ok_or_else(|| "Failed to convert Parabola".to_string())?;
                    let (p, q) = (inv_mat.transform_point(p), inv_mat.transform_point(q));
                    let (u, v) = (
                        UnitHyperbola::<Point2>::new()
                            .search_nearest_parameter(p, None, 0)
                            .ok_or_else(|| "the point is not on parabola".to_string())?,
                        UnitHyperbola::<Point2>::new()
                            .search_nearest_parameter(q, None, 0)
                            .ok_or_else(|| "the point is not on parabola".to_string())?,
                    );
                    let unit = TrimmedCurve::new(UnitHyperbola::<Point2>::new(), (u, v));
                    let mut parabola = Processor::new(unit);
                    parabola.transform_by(mat);
                    Curve2D::Conic(Conic2D::Hyperbola(parabola))
                }
            },
            CurveAny::Pcurve(_) => return Err("Pcurves cannot be parsed to 2D curves.".into()),
            CurveAny::SurfaceCurve(_) => {
                return Err("Surface curves cannot be parsed to 2D curves.".into())
            }
        };
        if !same_sense {
            curve.invert();
        }
        Ok(curve)
    }
    pub fn parse_curve3d(&self) -> Result<Curve3D, StepConvertingError> {
        let p = Point3::from(&self.edge_start.vertex_geometry);
        let q = Point3::from(&self.edge_end.vertex_geometry);
        let (p, q) = match self.same_sense {
            true => (p, q),
            false => (q, p),
        };
        Self::sub_parse_curve3d(&self.edge_geometry, p, q, self.same_sense)
    }
    fn sub_parse_curve3d(
        curve: &CurveAny,
        p: Point3,
        q: Point3,
        same_sense: bool,
    ) -> Result<Curve3D, StepConvertingError> {
        let ctx = ToleranceCtx::unscaled_legacy();
        let mut curve = match curve {
            CurveAny::Line(_) => Curve3D::Line(Line(p, q)),
            CurveAny::BoundedCurve(b) => b.as_ref().try_into()?,
            CurveAny::Conic(curve) => match curve.as_ref() {
                Conic::Circle(circle) => {
                    let mat =
                        Matrix4::try_from(&circle.position)? * Matrix4::from_scale(circle.radius);
                    let inv_mat = mat
                        .invert()
                        .ok_or_else(|| "Failed to convert Circle".to_string())?;
                    let (p, q) = (inv_mat.transform_point(p), inv_mat.transform_point(q));
                    let (u, mut v) = (
                        UnitCircle::<Point3>::new()
                            .search_nearest_parameter(p, None, 0)
                            .ok_or_else(|| format!("the point is not on circle: {p:?}"))?,
                        UnitCircle::<Point3>::new()
                            .search_nearest_parameter(q, None, 0)
                            .ok_or_else(|| format!("the point is not on circle: {q:?}"))?,
                    );
                    if v < u - ctx.ratio_margin() {
                        // BG-TOL-001: param
                        v += 2.0 * PI;
                    }
                    let circle = TrimmedCurve::new(UnitCircle::<Point3>::new(), (u, v));
                    let mut ellipse = Processor::new(circle);
                    ellipse.transform_by(mat);
                    // Source family retained here too. This is the path an
                    // `edge_curve` takes when its trim is recovered from the
                    // two vertex points, so it is the one nearly every real
                    // bound curve goes through -- routing only the plain
                    // `TryFrom` would have left the corpus unchanged.
                    Curve3D::Conic(Conic3D::Circle(ellipse))
                }
                Conic::Ellipse(ellipse) => {
                    let mat = Matrix4::try_from(&ellipse.position)?
                        * Matrix4::from_nonuniform_scale(
                            ellipse.semi_axis_1,
                            ellipse.semi_axis_2,
                            f64::min(ellipse.semi_axis_1, ellipse.semi_axis_2),
                        );
                    let inv_mat = mat
                        .invert()
                        .ok_or_else(|| "Failed to convert Circle".to_string())?;
                    let (p, q) = (inv_mat.transform_point(p), inv_mat.transform_point(q));
                    let (u, mut v) = (
                        UnitCircle::<Point3>::new()
                            .search_nearest_parameter(p, None, 0)
                            .ok_or_else(|| format!("the point is not on circle: {p:?}"))?,
                        UnitCircle::<Point3>::new()
                            .search_nearest_parameter(q, None, 0)
                            .ok_or_else(|| format!("the point is not on circle: {q:?}"))?,
                    );
                    if v < u - ctx.ratio_margin() {
                        // BG-TOL-001: param
                        v += 2.0 * PI;
                    }
                    let circle = TrimmedCurve::new(UnitCircle::<Point3>::new(), (u, v));
                    let mut ellipse = Processor::new(circle);
                    ellipse.transform_by(mat);
                    Curve3D::Conic(Conic3D::Ellipse(ellipse))
                }
                Conic::Hyperbola(hyperbola) => {
                    let mat = Matrix4::try_from(&hyperbola.position)?
                        * Matrix4::from_nonuniform_scale(
                            hyperbola.semi_axis,
                            hyperbola.semi_imag_axis,
                            f64::min(hyperbola.semi_axis, hyperbola.semi_imag_axis),
                        );
                    let inv_mat = mat
                        .invert()
                        .ok_or_else(|| "Failed to convert Circle".to_string())?;
                    let (p, q) = (inv_mat.transform_point(p), inv_mat.transform_point(q));
                    let (u, v) = (
                        UnitHyperbola::<Point3>::new()
                            .search_nearest_parameter(p, None, 0)
                            .ok_or_else(|| "the point is not on circle".to_string())?,
                        UnitHyperbola::<Point3>::new()
                            .search_nearest_parameter(q, None, 0)
                            .ok_or_else(|| "the point is not on circle".to_string())?,
                    );
                    let unit = TrimmedCurve::new(UnitHyperbola::<Point3>::new(), (u, v));
                    let mut hyperbola = Processor::new(unit);
                    hyperbola.transform_by(mat);
                    Curve3D::Conic(Conic3D::Hyperbola(hyperbola))
                }
                Conic::Parabola(parabola) => {
                    let mat = Matrix4::try_from(&parabola.position)?
                        * Matrix4::from_scale(parabola.focal_dist);
                    let inv_mat = mat
                        .invert()
                        .ok_or_else(|| "Failed to convert Parabola".to_string())?;
                    let (p, q) = (inv_mat.transform_point(p), inv_mat.transform_point(q));
                    let (u, v) = (
                        UnitHyperbola::<Point3>::new()
                            .search_nearest_parameter(p, None, 0)
                            .ok_or_else(|| "the point is not on parabola".to_string())?,
                        UnitHyperbola::<Point3>::new()
                            .search_nearest_parameter(q, None, 0)
                            .ok_or_else(|| "the point is not on parabola".to_string())?,
                    );
                    let unit = TrimmedCurve::new(UnitHyperbola::<Point3>::new(), (u, v));
                    let mut parabola = Processor::new(unit);
                    parabola.transform_by(mat);
                    Curve3D::Conic(Conic3D::Hyperbola(parabola))
                }
            },
            CurveAny::Pcurve(c) => {
                let surface: Surface = (&c.basis_surface).try_into()?;
                let u = surface
                    .search_nearest_parameter(p, None, 100)
                    .ok_or_else(|| "the point is not on surface".to_string())?;
                let v = surface
                    .search_nearest_parameter(q, None, 100)
                    .ok_or_else(|| "the point is not on surface".to_string())?;
                let curve2d = c
                    .reference_to_curve
                    .representation_item
                    .first()
                    .ok_or("no representation item")?;
                let curve2d = Self::sub_parse_2d(
                    curve2d,
                    Point2::new(u.0, u.1),
                    Point2::new(v.0, v.1),
                    true,
                )?;
                Curve3D::PCurve(truck::PCurve::new(Box::new(curve2d), Box::new(surface)))
            }
            CurveAny::SurfaceCurve(c) => {
                if ctx.near_pt(p, q) {
                    // BG-TOL-001: model
                    return Self::sub_parse_curve3d(&c.curve_3d, p, q, same_sense);
                }
                use PreferredSurfaceCurveRepresentation::*;
                match c.master_representation {
                    Curve3D => Self::sub_parse_curve3d(&c.curve_3d, p, q, same_sense)?,
                    PcurveS1 => {
                        if let Some(PcurveOrSurface::Pcurve(c)) = c.associated_geometry.first() {
                            Self::sub_parse_curve3d(&CurveAny::Pcurve(c.clone()), p, q, true)?
                        } else {
                            return Err(
                                "The 0-indexed associated geometry is nothing or not PCURVE."
                                    .into(),
                            );
                        }
                    }
                    PcurveS2 => {
                        if let Some(PcurveOrSurface::Pcurve(c)) = c.associated_geometry.get(1) {
                            Self::sub_parse_curve3d(&CurveAny::Pcurve(c.clone()), p, q, true)?
                        } else {
                            return Err(
                                "The 1-indexed associated geometry is nothing or not PCURVE."
                                    .into(),
                            );
                        }
                    }
                }
            }
        };
        if !same_sense {
            curve.invert();
        }
        Ok(curve)
    }
}

/// `oriented_edge`
///
/// `oriented_edge` has duplicated information.
/// These are not included here because they are essentially omitted.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = oriented_edge)]
#[holder(generate_deserialize)]
pub struct OrientedEdge {
    pub label: String,
    #[holder(use_place_holder)]
    pub edge_element: EdgeCurve,
    pub orientation: bool,
}

impl OrientedEdgeHolder {
    fn edge_element_holder(&self, table: &Table) -> Option<EdgeCurveHolder> {
        match &self.edge_element {
            PlaceHolder::Owned(holder) => Some(holder.clone()),
            PlaceHolder::Ref(Name::Entity(idx)) => table.edge_curve.get(idx).cloned(),
            _ => None,
        }
    }
    fn edge_element_idx(&self) -> Option<u64> {
        if let PlaceHolder::Ref(Name::Entity(idx)) = self.edge_element {
            Some(idx)
        } else {
            None
        }
    }
}

/// `edge_loop`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = edge_loop)]
#[holder(generate_deserialize)]
pub struct EdgeLoop {
    pub label: String,
    #[holder(use_place_holder)]
    pub edge_list: Vec<EdgeAny>,
}

/// Which kind of loop a `FACE_BOUND` resolved to.
///
/// The two are not interchangeable and must not be collapsed to "a loop". An
/// `EDGE_LOOP` contributes a trim curve; a `VERTEX_LOOP` contributes none and
/// instead marks a point where the chart itself is singular (`QUO-005`). Code
/// that treats the second as an empty instance of the first would synthesise a
/// zero-length boundary and trim the face by nothing.
#[derive(Clone, Debug, PartialEq)]
pub enum FaceBoundLoop {
    /// An ordinary loop of oriented edges.
    Edges(EdgeLoopHolder),
    /// A boundary collapsed to one vertex: a cone apex or a sphere pole.
    Collapsed(VertexLoopHolder),
}

/// `vertex_loop`
///
/// A loop that is a single vertex and has no edges: the collapsed boundary at a
/// cone apex or a sphere pole, where the surface's own parameterisation closes
/// the domain and there is no curve to trim along.
///
/// Unsupported until 2026-07-29, and it was the single largest cause of missing
/// faces in both corpora -- 272 of 604 on ABC `00009190` and 132 across NIST,
/// with the entity count matching the failure count exactly in all eight files
/// that contain one. `FaceBoundHolder::bound_holder` resolved a bound only
/// against `edge_loop`, so a face with an apex lost its whole self.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = vertex_loop)]
#[holder(generate_deserialize)]
pub struct VertexLoop {
    pub label: String,
    #[holder(use_place_holder)]
    pub loop_vertex: VertexPoint,
}

/// `face_bound`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = face_bound)]
#[holder(generate_deserialize)]
/// `FACE_OUTER_BOUNDS` is also parsed to this struct.
pub struct FaceBound {
    pub label: String,
    // For now, we are going with the policy of accepting nothing but edgeloop.
    #[holder(use_place_holder)]
    pub bound: EdgeLoop,
    pub orientation: bool,
}

impl FaceBoundHolder {
    /// What kind of loop this bound actually names.
    ///
    /// STEP permits a face bound to reference either an `EDGE_LOOP` or a
    /// `VERTEX_LOOP`; the reference itself is untyped, so which one it is can
    /// only be discovered by looking. This used to check `edge_loop` alone and
    /// return `None` otherwise, which turned every apex into a lost face.
    fn bound_holder(&self, table: &Table) -> Option<FaceBoundLoop> {
        match &self.bound {
            PlaceHolder::Owned(holder) => Some(FaceBoundLoop::Edges(holder.clone())),
            PlaceHolder::Ref(Name::Entity(ref idx)) => table
                .edge_loop
                .get(idx)
                .cloned()
                .map(FaceBoundLoop::Edges)
                .or_else(|| {
                    table
                        .vertex_loop
                        .get(idx)
                        .cloned()
                        .map(FaceBoundLoop::Collapsed)
                }),
            _ => None,
        }
    }
}

/// `face`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(generate_deserialize)]
pub enum FaceAny {
    #[holder(use_place_holder)]
    FaceSurface(FaceSurface),
    #[holder(use_place_holder)]
    OrientedFace(OrientedFace),
}

/// `face_surface`
///
/// `advanced_face` is also parsed to this struct.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = face_surface)]
#[holder(generate_deserialize)]
pub struct FaceSurface {
    pub label: String,
    #[holder(use_place_holder)]
    pub bounds: Vec<FaceBound>,
    #[holder(use_place_holder)]
    pub face_geometry: SurfaceAny,
    pub same_sense: bool,
}

impl FaceSurfaceHolder {
    /// Whether each bound arrived as a `FACE_OUTER_BOUND`, in `bounds` order.
    ///
    /// `None` for a bound the document *inlined*: the entity type was known
    /// while the record was being read and is not recoverable from the
    /// embedded holder, so the honest answer is that this reader does not
    /// know. A caller that meets one must decline to claim standing for the
    /// whole face rather than read `None` as "not outer".
    fn bound_outer_flags(&self, table: &Table) -> Vec<Option<bool>> {
        self.bounds
            .iter()
            .map(|bound| match bound {
                PlaceHolder::Ref(Name::Entity(idx)) => {
                    Some(table.face_outer_bound_ids.contains(idx))
                }
                _ => None,
            })
            .collect()
    }

    fn bounds_holder<'a>(&'a self, table: &'a Table) -> Vec<Option<FaceBoundHolder>> {
        self.bounds
            .iter()
            .map(|bound| match bound {
                PlaceHolder::Owned(bound) => Some(bound.clone()),
                PlaceHolder::Ref(Name::Entity(ref idx)) => table.face_bound.get(idx).cloned(),
                _ => None,
            })
            .collect()
    }
}

/// `oriented_face`
///
/// `oriented_face` has duplicated information.
/// These are not included here because they are essentially omitted.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = oriented_face)]
#[holder(generate_deserialize)]
pub struct OrientedFace {
    pub label: String,
    #[holder(use_place_holder)]
    pub face_element: FaceSurface,
    pub orientation: bool,
}

impl OrientedFaceHolder {
    fn face_element_holder(&self, table: &Table) -> Option<FaceSurfaceHolder> {
        match &self.face_element {
            PlaceHolder::Ref(Name::Entity(ref idx)) => table.face_surface.get(idx).cloned(),
            PlaceHolder::Owned(x) => Some(x.clone()),
            _ => None,
        }
    }
}

/// `shell`
///
/// Includes `open_shell` and `closed_shell`.
/// Since these differences are only informal propositions, the data structure does not distinguish between the two.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = shell)]
#[holder(generate_deserialize)]
pub struct Shell {
    pub label: String,
    #[holder(use_place_holder)]
    pub cfs_faces: Vec<FaceAny>,
}

impl ShellHolder {
    /// The shell's faces, each paired with the entity the shell named to reach
    /// it.
    ///
    /// The id is what `cfs_faces` *names* — an `ORIENTED_FACE` where the file
    /// used one, otherwise the `FACE_SURFACE`. That is the honest provenance:
    /// it is the reference a failure report should quote back, because it is
    /// the one a reader can find in the file. An inline owned face has no id,
    /// and gets `None` rather than a fabricated one.
    fn cfs_faces_holder<'a>(
        &'a self,
        table: &'a Table,
    ) -> impl Iterator<Item = (Option<u64>, Option<FaceAnyHolder>)> + 'a {
        self.cfs_faces.iter().map(|face| match face {
            PlaceHolder::Owned(holder) => (None, Some(holder.clone())),
            PlaceHolder::Ref(Name::Entity(ref idx)) => (
                Some(*idx),
                table
                    .oriented_face
                    .get(idx)
                    .cloned()
                    .map(FaceAnyHolder::OrientedFace)
                    .or_else(|| {
                        table
                            .face_surface
                            .get(idx)
                            .cloned()
                            .map(FaceAnyHolder::FaceSurface)
                    }),
            ),
            _ => (None, None),
        })
    }
}

/// `oriented_shell`
///
/// Includes `oriented_open_shell` and `oriented_closed_shell`.
/// Since these differences are only informal propositions, the data structure does not distinguish between the two.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = oriented_shell)]
#[holder(generate_deserialize)]
pub struct OrientedShell {
    pub label: String,
    #[holder(use_place_holder)]
    pub shell_element: Shell,
    pub orientation: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(generate_deserialize)]
pub enum ShellAny {
    #[holder(use_place_holder)]
    Shell(Shell),
    #[holder(use_place_holder)]
    OrientedShell(OrientedShell),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = shell_based_surface_model)]
#[holder(generate_deserialize)]
pub struct ShellBasedSurfaceModel {
    pub label: String,
    #[holder(use_place_holder)]
    pub sbsm_boundary: Vec<ShellAny>,
}

/// Also serves as `brep_with_voids`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = manifold_solid_brep)]
#[holder(generate_deserialize)]
pub struct ManifoldSolidBrep {
    pub label: String,
    #[holder(use_place_holder)]
    pub outer: ShellAny,
    #[holder(use_place_holder)]
    pub voids: Vec<OrientedShell>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = application_context)]
#[holder(generate_deserialize)]
pub struct ApplicationContext {
    pub application: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = product_context)]
#[holder(generate_deserialize)]
pub struct ProductContext {
    pub name: String,
    #[holder(use_place_holder)]
    pub frame_of_reference: ApplicationContext,
    pub discipline_type: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = product)]
#[holder(generate_deserialize)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub description: String,
    #[holder(use_place_holder)]
    pub frame_of_reference: Vec<ProductContext>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = product_definition_formation)]
#[holder(generate_deserialize)]
pub struct ProductDefinitionFormation {
    pub id: String,
    pub description: String,
    #[holder(use_place_holder)]
    pub of_product: Product,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = product_definition_context)]
#[holder(generate_deserialize)]
pub struct ProductDefinitionContext {
    pub name: String,
    #[holder(use_place_holder)]
    pub frame_of_reference: ApplicationContext,
    pub life_cycle_stage: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = product_definition)]
#[holder(generate_deserialize)]
pub struct ProductDefinition {
    pub id: String,
    pub description: String,
    #[holder(use_place_holder)]
    pub formation: ProductDefinitionFormation,
    #[holder(use_place_holder)]
    pub frame_of_reference: ProductDefinitionContext,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(generate_deserialize)]
pub enum CharacterizedDefinition {
    #[holder(use_place_holder)]
    ProductDefinition(Box<ProductDefinition>),
    #[holder(use_place_holder)]
    ProductDefinitionShape(Box<ProductDefinitionShape>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = product_definition_shape)]
#[holder(generate_deserialize)]
pub struct ProductDefinitionShape {
    pub name: String,
    pub description: String,
    #[holder(use_place_holder)]
    pub definition: CharacterizedDefinition,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = shape_representation)]
#[holder(generate_deserialize)]
pub struct ShapeRepresentation {
    pub name: String,
    #[holder(use_place_holder)]
    pub items: Vec<RepresentationItem>,
    #[holder(use_place_holder)]
    pub context_of_items: RepresentationContext,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = context_dependent_shape_representation)]
#[holder(generate_deserialize)]
pub struct ContextDependentShapeRepresentation {
    #[holder(use_place_holder)]
    pub representation_relation: ShapeRepresentationRelationshipWithTransformation,
    #[holder(use_place_holder)]
    pub represented_product_relation: ProductDefinitionShape,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = shape_definition_representation)]
#[holder(generate_deserialize)]
pub struct ShapeDefinitionRepresentation {
    #[holder(use_place_holder)]
    pub definition: ProductDefinitionShape,
    #[holder(use_place_holder)]
    pub used_representation: ShapeRepresentation,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = shape_representation_relationship)]
#[holder(generate_deserialize)]
pub struct ShapeRepresentationRelationship {
    pub name: String,
    pub description: String,
    #[holder(use_place_holder)]
    pub rep_1: ShapeRepresentation,
    #[holder(use_place_holder)]
    pub rep_2: ShapeRepresentation,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = shape_representation_relationship_with_transformation)]
#[holder(generate_deserialize)]
pub struct ShapeRepresentationRelationshipWithTransformation {
    pub name: String,
    pub description: String,
    #[holder(use_place_holder)]
    pub rep_1: ShapeRepresentation,
    #[holder(use_place_holder)]
    pub rep_2: ShapeRepresentation,
    #[holder(use_place_holder)]
    pub transformation_operator: ItemDefinedTransformation,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = next_assembly_usage_occurrence)]
#[holder(generate_deserialize)]
pub struct NextAssemblyUsageOccurrence {
    pub id: String,
    pub name: String,
    pub description: String,
    #[holder(use_place_holder)]
    pub relating_product_definition: ProductDefinition,
    #[holder(use_place_holder)]
    pub related_product_definition: ProductDefinition,
    pub reference_designator: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = item_defined_transformation)]
#[holder(generate_deserialize)]
pub struct ItemDefinedTransformation {
    name: String,
    description: String,
    #[holder(use_place_holder)]
    transform_item_1: Axis2Placement,
    #[holder(use_place_holder)]
    transform_item_2: Axis2Placement,
}

/// The placement being transformed *from* has to be inverted, and a singular
/// one cannot be. This is defensive rather than a fix for an observed failure:
/// the degenerate-placement handling above resolves every basis to an
/// orthonormal one, so there may be no input that reaches it today. It is still
/// wrong to unwrap inside a conversion that returns a `Result` — if anything
/// ever does reach it, aborting the process is not the intended contract.
const SINGULAR_TRANSFORM: &str = "ITEM_DEFINED_TRANSFORMATION has a degenerate source placement";

impl TryFrom<&ItemDefinedTransformation> for Matrix3 {
    type Error = StepConvertingError;
    fn try_from(value: &ItemDefinedTransformation) -> Result<Self, Self::Error> {
        let mat1: Self = (&value.transform_item_1).try_into()?;
        let mat2: Self = (&value.transform_item_2).try_into()?;
        Ok(mat2 * mat1.invert().ok_or(SINGULAR_TRANSFORM)?)
    }
}

impl TryFrom<&ItemDefinedTransformation> for Matrix4 {
    type Error = StepConvertingError;
    fn try_from(value: &ItemDefinedTransformation) -> Result<Self, Self::Error> {
        let mat1: Self = (&value.transform_item_1).try_into()?;
        let mat2: Self = (&value.transform_item_2).try_into()?;
        Ok(mat2 * mat1.invert().ok_or(SINGULAR_TRANSFORM)?)
    }
}

#[cfg(test)]
mod degenerate_placement_tests {
    use super::*;

    fn direction(x: f64, y: f64, z: f64) -> Direction {
        Direction {
            label: String::new(),
            direction_ratios: vec![x, y, z],
        }
    }

    fn placement(axis: Option<Direction>, ref_direction: Option<Direction>) -> Axis2Placement3d {
        Axis2Placement3d {
            label: String::new(),
            location: CartesianPoint {
                label: String::new(),
                coordinates: vec![0.0, 0.0, 0.0],
            },
            axis,
            ref_direction,
        }
    }

    fn assert_orthonormal(matrix: Matrix4) {
        for column in 0..4 {
            for row in 0..4 {
                assert!(
                    matrix[column][row].is_finite(),
                    "matrix must be finite, got {matrix:?}"
                );
            }
        }
        let x = matrix[0].truncate();
        let y = matrix[1].truncate();
        let z = matrix[2].truncate();
        for (name, vector) in [("x", x), ("y", y), ("z", z)] {
            assert!(
                (vector.magnitude() - 1.0).abs() < 1.0e-9,
                "{name} axis should be unit, got {}",
                vector.magnitude()
            );
        }
        assert!(x.dot(y).abs() < 1.0e-9, "x and y should be orthogonal");
        assert!(y.dot(z).abs() < 1.0e-9, "y and z should be orthogonal");
        assert!(z.dot(x).abs() < 1.0e-9, "z and x should be orthogonal");
    }

    /// A reference direction parallel to the axis leaves nothing to project,
    /// which used to normalize the zero vector into NaN.
    #[test]
    fn parallel_reference_direction_stays_finite() {
        let matrix = Matrix4::from(&placement(
            Some(direction(0.0, 0.0, 1.0)),
            Some(direction(0.0, 0.0, 1.0)),
        ));
        assert_orthonormal(matrix);
    }

    #[test]
    fn antiparallel_reference_direction_stays_finite() {
        let matrix = Matrix4::from(&placement(
            Some(direction(0.0, 0.0, 1.0)),
            Some(direction(0.0, 0.0, -1.0)),
        ));
        assert_orthonormal(matrix);
    }

    #[test]
    fn zero_axis_falls_back_to_a_valid_basis() {
        let matrix = Matrix4::from(&placement(
            Some(direction(0.0, 0.0, 0.0)),
            Some(direction(1.0, 0.0, 0.0)),
        ));
        assert_orthonormal(matrix);
    }

    #[test]
    fn non_unit_directions_are_normalized() {
        let matrix = Matrix4::from(&placement(
            Some(direction(0.0, 0.0, 7.5)),
            Some(direction(3.2, 0.0, 0.0)),
        ));
        assert_orthonormal(matrix);
    }

    /// The ordinary case must keep its exact orientation.
    #[test]
    fn well_formed_placement_is_unchanged() {
        let matrix = Matrix4::from(&placement(
            Some(direction(0.0, 0.0, 1.0)),
            Some(direction(1.0, 0.0, 0.0)),
        ));
        assert_orthonormal(matrix);
        assert!((matrix[0].truncate() - Vector3::unit_x()).magnitude() < 1.0e-12);
        assert!((matrix[2].truncate() - Vector3::unit_z()).magnitude() < 1.0e-12);
    }
}

/// Malformed geometry has to travel back as an error rather than unwinding.
///
/// These conversions all return a `Result` already, so a panic inside one is
/// never the intended contract. It also cannot be contained: they run on rayon
/// workers under callers that abort on panic, so one bad face in a large
/// assembly used to take down the whole load instead of costing its own shell.
#[cfg(test)]
mod malformed_geometry_tests {
    use super::*;

    fn point(x: f64, y: f64, z: f64) -> CartesianPoint {
        CartesianPoint {
            label: String::new(),
            coordinates: vec![x, y, z],
        }
    }

    fn curve_with_knots(knots: Vec<f64>, multiplicities: Vec<i64>) -> BSplineCurveWithKnots {
        BSplineCurveWithKnots {
            label: String::new(),
            degree: 1,
            control_points_list: vec![point(0.0, 0.0, 0.0), point(1.0, 0.0, 0.0)],
            curve_form: BSplineCurveForm::Unspecified,
            closed_curve: Logical::False,
            self_intersect: Logical::False,
            knot_multiplicities: multiplicities,
            knots,
            knot_spec: KnotType::Unspecified,
        }
    }

    /// Exporters emit knot vectors that are not monotonically increasing.
    #[test]
    fn unsorted_knots_report_rather_than_panic() {
        let curve = curve_with_knots(vec![0.0, 1.0, 0.5], vec![2, 1, 2]);
        let converted = BSplineCurve::<Point3>::try_from(&curve);
        assert!(converted.is_err(), "an unsorted knot vector must report");
    }

    /// The ordinary case still converts.
    #[test]
    fn sorted_knots_still_convert() {
        let curve = curve_with_knots(vec![0.0, 1.0], vec![2, 2]);
        assert!(BSplineCurve::<Point3>::try_from(&curve).is_ok());
    }
}

#[cfg(test)]
mod parameter_value_conversion_tests {
    use super::*;

    #[test]
    fn test_context_sensitive_parameter_conversion() {
        let deg_to_rad = std::f64::consts::PI / 180.0;

        // PlaneAngle dimension converts degrees to radians
        assert!(
            (convert_parameter_value(90.0, ParameterDimension::PlaneAngle, deg_to_rad)
                - std::f64::consts::FRAC_PI_2)
                .abs()
                < 1e-12
        );
        assert!(
            (convert_parameter_value(180.0, ParameterDimension::PlaneAngle, deg_to_rad)
                - std::f64::consts::PI)
                .abs()
                < 1e-12
        );

        // Length and Dimensionless parameters are untouched regardless of plane angle unit factor
        assert!(
            (convert_parameter_value(90.0, ParameterDimension::Length, deg_to_rad) - 90.0).abs()
                < 1e-12
        );
        assert!(
            (convert_parameter_value(90.0, ParameterDimension::Dimensionless, deg_to_rad) - 90.0)
                .abs()
                < 1e-12
        );
        assert!(
            (convert_parameter_value(90.0, ParameterDimension::NativeCurveParameter, deg_to_rad)
                - 90.0)
                .abs()
                < 1e-12
        );
    }
}

#[cfg(test)]
mod source_geometric_uncertainty_tests {
    use super::*;

    /// A single-solid table, hand-built exactly as the parser would fill it:
    /// shell #5 owned by solid #4, representation #6 naming that solid and
    /// declaring context #3, context #3 assigned uncertainty measure #2.
    ///
    /// Built by hand rather than parsed because the resolution logic is what is
    /// under test; the real-file parsing is exercised by the corpus sweeps.
    fn table_with_uncertainty() -> Table {
        let mut table = Table::default();
        table.shell.insert(
            5,
            ShellHolder {
                label: String::new(),
                cfs_faces: Vec::new(),
            },
        );
        table.manifold_solid_brep.insert(
            4,
            ManifoldSolidBrepHolder {
                label: String::new(),
                outer: PlaceHolder::Ref(Name::Entity(5)),
                voids: Vec::new(),
            },
        );
        table.shape_representation.insert(
            6,
            ShapeRepresentationHolder {
                name: String::new(),
                items: vec![PlaceHolder::Ref(Name::Entity(4))],
                context_of_items: PlaceHolder::Ref(Name::Entity(3)),
            },
        );
        table.uncertainty_measures.insert(2, 5.0e-3);
        table
            .global_uncertainty_assigned_contexts
            .insert(3, vec![2]);
        table
    }

    /// The value the file declares is the value the shell's representation is
    /// resolved to, in the file's native units.
    #[test]
    fn a_typed_length_uncertainty_resolves_to_its_value() {
        let table = table_with_uncertainty();
        let uncertainty = table.source_geometric_uncertainty(5);
        assert_eq!(uncertainty, Some(5.0e-3));
    }

    /// The chain survives an oriented-shell indirection: a solid may name an
    /// `ORIENTED_CLOSED_SHELL` rather than the shell directly.
    #[test]
    fn an_oriented_shell_reference_still_resolves() {
        let mut table = table_with_uncertainty();
        table.oriented_shell.insert(
            40,
            OrientedShellHolder {
                label: String::new(),
                shell_element: PlaceHolder::Ref(Name::Entity(5)),
                orientation: true,
            },
        );
        table.manifold_solid_brep.insert(
            4,
            ManifoldSolidBrepHolder {
                label: String::new(),
                outer: PlaceHolder::Ref(Name::Entity(40)),
                voids: Vec::new(),
            },
        );
        assert_eq!(table.source_geometric_uncertainty(5), Some(5.0e-3));
    }

    /// A shell that no representation owns has no applicable uncertainty: the
    /// honest answer is `None`, not an invented number.
    #[test]
    fn an_owned_shell_without_a_declared_uncertainty_is_none() {
        let mut table = table_with_uncertainty();
        table.global_uncertainty_assigned_contexts.clear();
        assert_eq!(table.source_geometric_uncertainty(5), None);
    }

    /// A shell that is not referenced by any solid or representation at all has
    /// no applicable uncertainty either.
    #[test]
    fn an_unowned_shell_is_none() {
        let table = table_with_uncertainty();
        assert_eq!(table.source_geometric_uncertainty(999), None);
    }

    /// A declared uncertainty that is unusable (non-finite or non-positive) is
    /// not passed on: the source supplied no usable value.
    #[test]
    fn a_non_positive_uncertainty_is_none() {
        let mut table = table_with_uncertainty();
        table.uncertainty_measures.insert(2, 0.0);
        assert_eq!(table.source_geometric_uncertainty(5), None);
        table.uncertainty_measures.insert(2, f64::NAN);
        assert_eq!(table.source_geometric_uncertainty(5), None);
    }
}
