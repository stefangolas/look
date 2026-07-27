use std::{env, fs, path::PathBuf, time::Instant};

use anyhow::Context;
use glam::{Mat4, Vec3};
use look::{
    camera::PreparedCamera,
    config::{LightingConfig, MaterialMode, RenderConfig, UpAxis},
    output::write_png,
    renderer::{Renderer, WgpuRenderer},
    scene::{Bounds, compile_scene_for_render, prepare_source_textures},
    timing::Timings,
};
use serde_json::json;

fn main() -> anyhow::Result<()> {
    let mut args = env::args_os().skip(1);
    let source = PathBuf::from(
        args.next()
            .context("usage: camera_path_benchmark MODEL [FRAMES] [RESOLUTION] [--antialias]")?,
    );
    let frames = args
        .next()
        .and_then(|value| value.to_str().and_then(|value| value.parse::<usize>().ok()))
        .unwrap_or(120)
        .max(4);
    let resolution = args
        .next()
        .and_then(|value| value.to_str().and_then(parse_resolution))
        .unwrap_or([1024, 1024]);
    let antialias = args
        .next()
        .is_some_and(|value| value.to_string_lossy() == "--antialias");

    let setup_started = Instant::now();
    let mut setup_timings = Timings::default();
    let mut scene =
        compile_scene_for_render(&source, UpAxis::Y, MaterialMode::Source, &mut setup_timings)?;
    prepare_source_textures(&mut scene, &mut setup_timings)?;
    let mut renderer = WgpuRenderer::new()?;
    let render = RenderConfig {
        resolution,
        material_mode: MaterialMode::Source,
        antialias,
        ..RenderConfig::default()
    };
    let cameras = camera_path(&scene.bounds, resolution, frames);
    renderer.render_views(&scene, &cameras[..1], &render, &LightingConfig::default())?;
    let setup_ms = setup_started.elapsed().as_secs_f64() * 1_000.0;

    let mut samples = Vec::with_capacity(frames);
    for (frame, camera) in cameras.iter().enumerate() {
        let started = Instant::now();
        let batch = renderer.render_views(
            &scene,
            std::slice::from_ref(camera),
            &render,
            &LightingConfig::default(),
        )?;
        samples.push(json!({
            "frame": frame,
            "wall_ms": started.elapsed().as_secs_f64() * 1_000.0,
            "gpu_render_ms": batch.timings.get("gpu_render"),
            "gpu_encode_submit_ms": batch.timings.get("gpu_encode_submit"),
            "gpu_readback_ms": batch.timings.get("gpu_readback"),
        }));
    }

    let keyframe_indices = [0, frames / 3, frames * 2 / 3, frames - 1];
    let keyframes = keyframe_indices.map(|index| cameras[index].clone());
    let atlas_render = RenderConfig {
        resolution: resolution.map(|dimension| dimension.min(1024)),
        antialias: false,
        atlas_columns: Some(2),
        ..render
    };
    let mut atlas = renderer.render_views(
        &scene,
        &keyframes,
        &atlas_render,
        &LightingConfig::default(),
    )?;
    let atlas = atlas.images.pop().context("renderer returned no atlas")?;
    let atlas_path = PathBuf::from("target/camera-path-benchmark/path-atlas.png");
    fs::create_dir_all(atlas_path.parent().unwrap())?;
    write_png(&atlas_path, &atlas)?;

    let wall = samples
        .iter()
        .filter_map(|sample| sample["wall_ms"].as_f64())
        .collect::<Vec<_>>();
    let gpu = samples
        .iter()
        .filter_map(|sample| sample["gpu_render_ms"].as_f64())
        .collect::<Vec<_>>();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "classification": "in-process GPU-resident camera path; wall clock includes render and readback but excludes PNG encoding",
            "source": source,
            "renderer": renderer.fingerprint(),
            "frames": frames,
            "resolution": resolution,
            "antialias": antialias,
            "setup_ms": setup_ms,
            "setup_timings_ms": setup_timings,
            "wall_ms": distribution(&wall),
            "gpu_render_ms": distribution(&gpu),
            "samples": samples,
            "keyframe_atlas": atlas_path,
        }))?
    );
    Ok(())
}

fn camera_path(bounds: &Bounds, resolution: [u32; 2], frames: usize) -> Vec<PreparedCamera> {
    let min = Vec3::from_array(bounds.min);
    let max = Vec3::from_array(bounds.max);
    let size = max - min;
    let center = (min + max) * 0.5;
    let aspect = resolution[0] as f32 / resolution[1] as f32;
    let projection = Mat4::perspective_rh(60.0_f32.to_radians(), aspect, 0.05, size.length() * 4.0);
    let start = center - Vec3::X * size.x * 0.32;
    let end = center + Vec3::X * size.x * 0.32;
    (0..frames)
        .map(|frame| {
            let t = frame as f32 / (frames - 1) as f32;
            let position = start.lerp(end, t)
                + Vec3::new(
                    0.0,
                    -size.y * 0.16 + (t * std::f32::consts::TAU).sin() * size.y * 0.03,
                    (t * std::f32::consts::TAU).sin() * size.z * 0.12,
                );
            let target = position
                + Vec3::new(
                    1.0,
                    (t * std::f32::consts::TAU).cos() * 0.08,
                    (t * std::f32::consts::TAU).cos() * 0.2,
                )
                .normalize();
            PreparedCamera {
                id: format!("path-{frame:04}"),
                view_projection: projection * Mat4::look_at_rh(position, target, Vec3::Y),
                position: position.to_array(),
                target: target.to_array(),
                up: Vec3::Y.to_array(),
            }
        })
        .collect()
}

fn distribution(samples: &[f64]) -> serde_json::Value {
    if samples.is_empty() {
        return serde_json::Value::Null;
    }
    let mut ordered = samples.to_vec();
    ordered.sort_by(f64::total_cmp);
    let percentile = |value: f64| ordered[((ordered.len() - 1) as f64 * value).round() as usize];
    json!({
        "min": ordered[0],
        "median": percentile(0.5),
        "p95": percentile(0.95),
        "max": ordered[ordered.len() - 1],
    })
}

fn parse_resolution(value: &str) -> Option<[u32; 2]> {
    let (width, height) = value.split_once('x')?;
    Some([width.parse().ok()?, height.parse().ok()?])
}
