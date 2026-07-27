use std::{fs, path::PathBuf};

use look::{
    camera::prepare_camera,
    config::{
        CameraKind, LightingConfig, MaterialMode, NamedView, RenderConfig, UpAxis, ViewConfig,
    },
    output::write_png,
    renderer::{Renderer, WgpuRenderer},
    scene::compile_glb,
    timing::Timings,
};

#[test]
#[ignore = "requires a native GPU"]
fn renders_generated_glb_to_png() {
    let artifact_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("gpu-smoke");
    fs::create_dir_all(&artifact_dir).unwrap();
    let model_path = artifact_dir.join("triangle.glb");
    let output_path = artifact_dir.join("triangle.png");
    fs::write(&model_path, triangle_glb()).unwrap();

    let mut timings = Timings::default();
    let scene = compile_glb(&model_path, UpAxis::Y, &mut timings).unwrap();
    assert_eq!(scene.statistics.triangles, 1);
    assert_eq!(scene.statistics.unique_geometries, 1);

    let render = RenderConfig {
        resolution: [128, 128],
        background: "#252525".to_owned(),
        base_color: "#d0d8e0".to_owned(),
        material_mode: MaterialMode::Technical,
        antialias: false,
        atlas_columns: None,
    };
    let view = ViewConfig::named(NamedView::Front, CameraKind::Orthographic);
    let camera = prepare_camera(&view, &scene.bounds, render.resolution);
    let cameras = [camera];
    unsafe { std::env::set_var("LOOK_GPU_TIMESTAMPS", "1") };
    let mut renderer = WgpuRenderer::new().unwrap();
    let batch = renderer
        .render_views(&scene, &cameras, &render, &LightingConfig::default())
        .unwrap();
    assert_eq!(batch.images.len(), 1);
    let image = &batch.images[0];
    let first_pixel = &image.rgba[0..4];
    assert!(
        image.rgba.chunks_exact(4).any(|pixel| pixel != first_pixel),
        "render contains only the background color"
    );

    write_png(&output_path, image).unwrap();
    assert!(fs::metadata(output_path).unwrap().len() > 100);

    let second = renderer
        .render_views(&scene, &cameras, &render, &LightingConfig::default())
        .unwrap();
    assert!(
        second.timings.get("gpu_upload").is_none(),
        "second render should reuse the GPU-resident scene"
    );

    let atlas_views = [
        NamedView::Front,
        NamedView::Right,
        NamedView::Top,
        NamedView::Iso,
    ]
    .map(|named| {
        prepare_camera(
            &ViewConfig::named(named, CameraKind::Orthographic),
            &scene.bounds,
            render.resolution,
        )
    });
    let atlas_render = RenderConfig {
        atlas_columns: Some(2),
        ..render
    };
    let atlas = renderer
        .render_views(
            &scene,
            &atlas_views,
            &atlas_render,
            &LightingConfig::default(),
        )
        .unwrap();
    assert_eq!(atlas.images.len(), 1);
    assert!(atlas.timings.get("gpu_render").is_some());
    assert_eq!([atlas.images[0].width, atlas.images[0].height], [256, 256]);
    assert_eq!(atlas.images[0].tiles.len(), 4);
    assert_eq!(
        atlas.images[0]
            .tiles
            .iter()
            .map(|tile| (tile.x, tile.y))
            .collect::<Vec<_>>(),
        vec![(0, 0), (128, 0), (0, 128), (128, 128)]
    );
}

#[test]
#[ignore = "requires a native GPU and mutates the process cache environment"]
fn persisted_server_session_round_trip() {
    let cache = tempfile::tempdir().unwrap();
    // This ignored integration test is run alone by CI, so changing the
    // process-wide environment cannot race another test.
    unsafe { std::env::set_var("LOOK_CACHE_DIR", cache.path()) };
    let server = std::thread::spawn(look::server::run_server);
    for _ in 0..200 {
        if look::server::server_state_path().unwrap().exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("gpu-smoke")
        .join("triangle.glb");
    if !fixture.exists() {
        fs::create_dir_all(fixture.parent().unwrap()).unwrap();
        fs::write(&fixture, triangle_glb()).unwrap();
    }
    let persisted =
        look::server::persist_session(fixture, UpAxis::Y, MaterialMode::Technical, 60).unwrap();
    let id = persisted["session"]["session_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let output = cache.path().join("session.png");
    let rendered = look::server::render_session(
        id.clone(),
        RenderConfig {
            resolution: [128, 128],
            ..RenderConfig::default()
        },
        LightingConfig::default(),
        vec![ViewConfig::named(
            NamedView::Front,
            CameraKind::Orthographic,
        )],
        look::config::OutputConfig {
            single_file: Some(output.clone()),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(rendered["timings_ms"]["gpu_upload"].is_null());
    assert!(output.is_file());
    look::server::close_session(id).unwrap();
    look::server::stop_server().unwrap();
    server.join().unwrap().unwrap();
}

fn triangle_glb() -> Vec<u8> {
    let mut binary = Vec::new();
    for value in [-1.0_f32, -1.0, 0.0, 1.0, -1.0, 0.0, 0.0, 1.0, 0.0] {
        binary.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0.0_f32, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0] {
        binary.extend_from_slice(&value.to_le_bytes());
    }
    for index in [0_u16, 1, 2] {
        binary.extend_from_slice(&index.to_le_bytes());
    }
    while binary.len() % 4 != 0 {
        binary.push(0);
    }

    let json = r#"{"asset":{"version":"2.0"},"scene":0,"scenes":[{"nodes":[0]}],"nodes":[{"mesh":0,"name":"Triangle"}],"meshes":[{"primitives":[{"attributes":{"POSITION":0,"NORMAL":1},"indices":2}]}],"buffers":[{"byteLength":80}],"bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":36},{"buffer":0,"byteOffset":36,"byteLength":36},{"buffer":0,"byteOffset":72,"byteLength":6}],"accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[-1,-1,0],"max":[1,1,0]},{"bufferView":1,"componentType":5126,"count":3,"type":"VEC3"},{"bufferView":2,"componentType":5123,"count":3,"type":"SCALAR"}]}"#;
    let mut json = json.as_bytes().to_vec();
    while !json.len().is_multiple_of(4) {
        json.push(b' ');
    }

    let total_length = 12 + 8 + json.len() + 8 + binary.len();
    let mut glb = Vec::with_capacity(total_length);
    glb.extend_from_slice(&0x4654_6c67_u32.to_le_bytes());
    glb.extend_from_slice(&2_u32.to_le_bytes());
    glb.extend_from_slice(&(total_length as u32).to_le_bytes());
    glb.extend_from_slice(&(json.len() as u32).to_le_bytes());
    glb.extend_from_slice(&0x4e4f_534a_u32.to_le_bytes());
    glb.extend_from_slice(&json);
    glb.extend_from_slice(&(binary.len() as u32).to_le_bytes());
    glb.extend_from_slice(&0x004e_4942_u32.to_le_bytes());
    glb.extend_from_slice(&binary);
    glb
}
