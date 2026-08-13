//! End-to-end STEP assembly scene organization: unique definition geometry
//! tessellated once, occurrence placement applied exactly once, repeated
//! definitions staying distinct occurrences, and nested composition obeying
//! `T_world(child) = T_world(parent) * T_local(child)`.
//!
//! The fixture mirrors the `core_xy.step` encoding (placement-only
//! representation linked to a `ADVANCED_BREP_SHAPE_REPRESENTATION` through a
//! `SHAPE_REPRESENTATION_RELATIONSHIP`) plus a nested subassembly level.

use look::step::{StepScene, parse_step_scene};
use look::timing::Timings;

const FIXTURE: &str = r#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('Fixture'),'2;1');
FILE_NAME('assembly_geometry_fixture','2026-01-01T00:00:00',(''),(''),
  '','','');
FILE_SCHEMA(('AUTOMOTIVE_DESIGN { 1 0 10303 214 1 1 1 1 }'));
ENDSEC;
DATA;
#1 = APPLICATION_CONTEXT('assembly context');
#3 = PRODUCT_CONTEXT('',#1,'mechanical');
#4 = PRODUCT_DEFINITION_CONTEXT('part definition',#1,'design');
#5 = REPRESENTATION_CONTEXT('Context #1','3D Context');
#10 = PRODUCT('assembly','assembly','',(#3));
#11 = PRODUCT_DEFINITION_FORMATION('','',#10);
#12 = PRODUCT_DEFINITION('','',#11,#4);
#13 = PRODUCT_DEFINITION_SHAPE('','',#12);
#20 = PRODUCT('partA','partA','',(#3));
#21 = PRODUCT_DEFINITION_FORMATION('','',#20);
#22 = PRODUCT_DEFINITION('','',#21,#4);
#23 = PRODUCT_DEFINITION_SHAPE('','',#22);
#30 = PRODUCT('partB','partB','',(#3));
#31 = PRODUCT_DEFINITION_FORMATION('','',#30);
#32 = PRODUCT_DEFINITION('','',#31,#4);
#33 = PRODUCT_DEFINITION_SHAPE('','',#32);
#40 = PRODUCT('partC','partC','',(#3));
#41 = PRODUCT_DEFINITION_FORMATION('','',#40);
#42 = PRODUCT_DEFINITION('','',#41,#4);
#43 = PRODUCT_DEFINITION_SHAPE('','',#42);
#50 = PRODUCT('partD','partD','',(#3));
#51 = PRODUCT_DEFINITION_FORMATION('','',#50);
#52 = PRODUCT_DEFINITION('','',#51,#4);
#53 = PRODUCT_DEFINITION_SHAPE('','',#52);
#60 = PRODUCT('sub','sub','',(#3));
#61 = PRODUCT_DEFINITION_FORMATION('','',#60);
#62 = PRODUCT_DEFINITION('','',#61,#4);
#63 = PRODUCT_DEFINITION_SHAPE('','',#62);
#100 = DIRECTION('',(0.,0.,1.));
#101 = DIRECTION('',(1.,0.,0.));
#102 = CARTESIAN_POINT('',(0.,0.,0.));
#103 = AXIS2_PLACEMENT_3D('',#102,#100,#101);
#104 = PLANE('',#103);
#105 = CARTESIAN_POINT('',(0.,0.,0.));
#106 = CARTESIAN_POINT('',(1.,0.,0.));
#107 = CARTESIAN_POINT('',(0.,1.,0.));
#108 = VERTEX_POINT('',#105);
#109 = VERTEX_POINT('',#106);
#110 = VERTEX_POINT('',#107);
#111 = DIRECTION('',(1.,0.,0.));
#112 = VECTOR('',#111,1.);
#113 = LINE('',#105,#112);
#114 = DIRECTION('',(-1.,1.,0.));
#115 = VECTOR('',#114,1.);
#116 = LINE('',#106,#115);
#117 = DIRECTION('',(0.,-1.,0.));
#118 = VECTOR('',#117,1.);
#119 = LINE('',#107,#118);
#120 = EDGE_CURVE('',#108,#109,#113,.T.);
#121 = EDGE_CURVE('',#109,#110,#116,.T.);
#122 = EDGE_CURVE('',#110,#108,#119,.T.);
#123 = ORIENTED_EDGE('',*,*,#120,.T.);
#124 = ORIENTED_EDGE('',*,*,#121,.T.);
#125 = ORIENTED_EDGE('',*,*,#122,.T.);
#126 = EDGE_LOOP('',(#123,#124,#125));
#127 = FACE_OUTER_BOUND('',#126,.T.);
#128 = ADVANCED_FACE('',(#127),#104,.T.);
#129 = CLOSED_SHELL('',(#128));
#130 = MANIFOLD_SOLID_BREP('',#129);
#131 = MANIFOLD_SOLID_BREP('',#129);
#132 = ADVANCED_BREP_SHAPE_REPRESENTATION('',(#130),#5);
#133 = ADVANCED_BREP_SHAPE_REPRESENTATION('',(#131),#5);
#134 = ADVANCED_BREP_SHAPE_REPRESENTATION('',(#141,#130),#5);
#140 = CARTESIAN_POINT('',(0.,0.,0.));
#141 = AXIS2_PLACEMENT_3D('',#140,#100,#101);
#143 = CARTESIAN_POINT('',(1.,0.,0.));
#144 = AXIS2_PLACEMENT_3D('',#143,#100,#101);
#145 = CARTESIAN_POINT('',(-1.,0.,0.));
#146 = AXIS2_PLACEMENT_3D('',#145,#100,#101);
#147 = CARTESIAN_POINT('',(2.,0.,0.));
#149 = AXIS2_PLACEMENT_3D('',#147,#100,#101);
#150 = CARTESIAN_POINT('',(0.,0.,1.));
#153 = AXIS2_PLACEMENT_3D('',#150,#100,#101);
#154 = CARTESIAN_POINT('',(5.,0.,0.));
#155 = AXIS2_PLACEMENT_3D('',#154,#100,#101);
#156 = CARTESIAN_POINT('',(1.,0.,0.));
#157 = AXIS2_PLACEMENT_3D('',#156,#100,#101);
#160 = SHAPE_REPRESENTATION('partA',(#141),#5);
#161 = SHAPE_REPRESENTATION('partB',(#141),#5);
#162 = SHAPE_REPRESENTATION('partC',(#141),#5);
#163 = SHAPE_REPRESENTATION('main',(#141,#144,#146,#149,#153,#155),#5);
#164 = SHAPE_REPRESENTATION('sub',(#141,#157),#5);
#170 = SHAPE_REPRESENTATION_RELATIONSHIP('','',#160,#132);
#171 = SHAPE_REPRESENTATION_RELATIONSHIP('','',#161,#133);
#180 = SHAPE_DEFINITION_REPRESENTATION(#23,#160);
#181 = SHAPE_DEFINITION_REPRESENTATION(#33,#161);
#182 = SHAPE_DEFINITION_REPRESENTATION(#43,#162);
#183 = SHAPE_DEFINITION_REPRESENTATION(#53,#134);
#184 = SHAPE_DEFINITION_REPRESENTATION(#13,#163);
#185 = SHAPE_DEFINITION_REPRESENTATION(#63,#164);
#600 = NEXT_ASSEMBLY_USAGE_OCCURRENCE('occA','','',#12,#22,'');
#601 = NEXT_ASSEMBLY_USAGE_OCCURRENCE('occB1','','',#12,#32,'');
#602 = NEXT_ASSEMBLY_USAGE_OCCURRENCE('occB2','','',#12,#32,'');
#603 = NEXT_ASSEMBLY_USAGE_OCCURRENCE('occC','','',#12,#42,'');
#604 = NEXT_ASSEMBLY_USAGE_OCCURRENCE('occD','','',#12,#52,'');
#605 = NEXT_ASSEMBLY_USAGE_OCCURRENCE('occSubD','','',#62,#52,'');
#606 = NEXT_ASSEMBLY_USAGE_OCCURRENCE('occSub','','',#12,#62,'');
#610 = PRODUCT_DEFINITION_SHAPE('','',#600);
#611 = PRODUCT_DEFINITION_SHAPE('','',#601);
#612 = PRODUCT_DEFINITION_SHAPE('','',#602);
#613 = PRODUCT_DEFINITION_SHAPE('','',#603);
#614 = PRODUCT_DEFINITION_SHAPE('','',#604);
#615 = PRODUCT_DEFINITION_SHAPE('','',#605);
#616 = PRODUCT_DEFINITION_SHAPE('','',#606);
#630 = ITEM_DEFINED_TRANSFORMATION('','',#141,#144);
#631 = ITEM_DEFINED_TRANSFORMATION('','',#141,#146);
#632 = ITEM_DEFINED_TRANSFORMATION('','',#141,#149);
#633 = ITEM_DEFINED_TRANSFORMATION('','',#141,#153);
#634 = ITEM_DEFINED_TRANSFORMATION('','',#141,#144);
#635 = ITEM_DEFINED_TRANSFORMATION('','',#141,#157);
#636 = ITEM_DEFINED_TRANSFORMATION('','',#141,#155);
#620 = ( REPRESENTATION_RELATIONSHIP(' ',' ',#160,#163)
  REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#630)
  SHAPE_REPRESENTATION_RELATIONSHIP() );
#621 = ( REPRESENTATION_RELATIONSHIP(' ',' ',#161,#163)
  REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#631)
  SHAPE_REPRESENTATION_RELATIONSHIP() );
#622 = ( REPRESENTATION_RELATIONSHIP(' ',' ',#161,#163)
  REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#632)
  SHAPE_REPRESENTATION_RELATIONSHIP() );
#623 = ( REPRESENTATION_RELATIONSHIP(' ',' ',#162,#163)
  REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#633)
  SHAPE_REPRESENTATION_RELATIONSHIP() );
#624 = ( REPRESENTATION_RELATIONSHIP(' ',' ',#134,#163)
  REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#634)
  SHAPE_REPRESENTATION_RELATIONSHIP() );
#625 = ( REPRESENTATION_RELATIONSHIP(' ',' ',#134,#164)
  REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#635)
  SHAPE_REPRESENTATION_RELATIONSHIP() );
#626 = ( REPRESENTATION_RELATIONSHIP(' ',' ',#164,#163)
  REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#636)
  SHAPE_REPRESENTATION_RELATIONSHIP() );
#700 = CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(#620,#610);
#701 = CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(#621,#611);
#702 = CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(#622,#612);
#703 = CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(#623,#613);
#704 = CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(#624,#614);
#705 = CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(#625,#615);
#706 = CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(#626,#616);
ENDSEC;
END-ISO-10303-21;
"#;

fn assembly() -> look::step::StepAssemblyScene {
    let mut timings = Timings::default();
    match parse_step_scene(FIXTURE.as_bytes(), &mut timings).expect("fixture must parse") {
        StepScene::Assembly(scene) => scene,
        StepScene::Flat(_) => {
            panic!("the fixture declares occurrences and must take the assembly path")
        }
    }
}

fn translation(world: &glam::Mat4) -> [f32; 3] {
    [world.w_axis.x, world.w_axis.y, world.w_axis.z]
}

/// The assembly scene keeps definition geometry and occurrences separate:
/// three geometry-bearing definitions, five renderable occurrences (the two
/// geometry-less nodes' occurrences carry no instance), six product nodes.
#[test]
fn assembly_scene_counts_are_graph_semantic() {
    let scene = assembly();
    assert_eq!(scene.nodes, 6, "root, sub, A, B, C, D");
    assert_eq!(
        scene.definitions.len(),
        3,
        "A, B and D carry definition geometry"
    );
    assert_eq!(
        scene.occurrences.len(),
        5,
        "A, B x2, D x2; C and sub render nothing"
    );
}

/// A repeated definition stays one geometry and produces distinct occurrences
/// with distinct world transforms.
#[test]
fn repeated_definition_shares_geometry_and_splits_occurrences() {
    let scene = assembly();
    let b_definition = scene
        .definitions
        .iter()
        .position(|definition| definition.node_name == "partB")
        .expect("partB definition must exist");
    let b_occurrences = scene
        .occurrences
        .iter()
        .filter(|occurrence| occurrence.definition == b_definition)
        .collect::<Vec<_>>();
    assert_eq!(b_occurrences.len(), 2, "two source occurrences of partB");
    assert_eq!(
        b_occurrences[0].definition, b_occurrences[1].definition,
        "both occurrences share the same definition geometry"
    );
    let first = translation(&b_occurrences[0].world);
    let second = translation(&b_occurrences[1].world);
    assert_ne!(first, second, "the two placements must differ");
    let mut xs = [first[0], second[0]];
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert!(
        (xs[0] - (-1.0)).abs() < 1.0e-3 && (xs[1] - 2.0).abs() < 1.0e-3,
        "the B placements must be the source frames, got {xs:?}"
    );
}

/// Nested composition: `T_world(child) = T_world(parent) * T_local(child)`.
///
/// partD occurs directly from the root at (1,0,0) and through the subassembly
/// at (5,0,0) then (1,0,0) — the nested world must be (6,0,0).
#[test]
fn nested_occurrence_composes_parent_and_local() {
    let scene = assembly();
    let d_definition = scene
        .definitions
        .iter()
        .position(|definition| definition.node_name == "partD")
        .expect("partD definition must exist");
    let mut xs = scene
        .occurrences
        .iter()
        .filter(|occurrence| occurrence.definition == d_definition)
        .map(|occurrence| translation(&occurrence.world)[0])
        .collect::<Vec<_>>();
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(xs.len(), 2, "partD occurs directly and nested");
    assert!(
        (xs[0] - 1.0).abs() < 1.0e-3 && (xs[1] - 6.0).abs() < 1.0e-3,
        "the nested world must equal parent (5,0,0) times local (1,0,0), got {xs:?}"
    );
}

/// Multiplicity: definition vertices stay definition-local and the occurrence
/// transform is applied exactly once. partD's definition soup must sit at the
/// origin triangle, not at its occurrence placement.
#[test]
fn definition_geometry_stays_local_and_transform_applies_once() {
    let scene = assembly();
    let d_definition = scene
        .definitions
        .iter()
        .position(|definition| definition.node_name == "partD")
        .expect("partD definition must exist");
    let (positions, _, _) = &scene.definitions[d_definition].soup;
    let mut max_extent = 0.0_f32;
    for position in positions {
        max_extent = max_extent
            .max(position[0])
            .max(position[1])
            .max(position[2]);
    }
    assert!(
        max_extent < 1.5,
        "partD's tessellated definition must stay at the origin triangle (extent {max_extent})"
    );
    let world = &scene.occurrences[scene
        .occurrences
        .iter()
        .position(|occurrence| occurrence.definition == d_definition)
        .unwrap()];
    let t = translation(&world.world);
    assert!(
        (t[0] - 1.0).abs() < 1.0e-3,
        "the direct occurrence world transform is the source placement, applied once"
    );
}

/// A plain single-part file keeps the flat single-geometry scene.
#[test]
fn single_part_step_stays_flat() {
    let bytes = std::fs::read(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("bracket.step"),
    )
    .expect("bracket fixture must exist");
    let mut timings = Timings::default();
    let scene = parse_step_scene(&bytes, &mut timings).expect("bracket must parse");
    assert!(
        matches!(scene, StepScene::Flat(_)),
        "a file with no occurrences must use the single-part path"
    );
}
