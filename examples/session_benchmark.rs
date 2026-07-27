use std::{env, fs, path::PathBuf, time::Instant};

use anyhow::Context;
use look::{
    camera::prepare_camera,
    config::{
        CameraKind, LightingConfig, MaterialMode, NamedView, RenderConfig, UpAxis, ViewConfig,
    },
    output::write_png,
    renderer::{Renderer, WgpuRenderer},
    scene::{compile_scene_for_render, prepare_source_textures},
    timing::Timings,
};
use serde_json::json;

fn main() -> anyhow::Result<()> {
    let mut args = env::args_os().skip(1);
    let source = PathBuf::from(
        args.next()
            .context("usage: session_benchmark MODEL [ITERATIONS]")?,
    );
    let iterations = args
        .next()
        .and_then(|value| value.to_str().and_then(|value| value.parse::<usize>().ok()))
        .unwrap_or(11)
        .max(3);
    let output_dir = PathBuf::from("target/session-benchmark");
    fs::create_dir_all(&output_dir)?;

    let setup_started = Instant::now();
    let mut compile_timings = Timings::default();
    let mut scene = compile_scene_for_render(
        &source,
        UpAxis::Y,
        MaterialMode::Source,
        &mut compile_timings,
    )?;
    prepare_source_textures(&mut scene, &mut compile_timings)?;
    let mut renderer = WgpuRenderer::new()?;
    let warm_render = RenderConfig {
        resolution: [1, 1],
        material_mode: MaterialMode::Source,
        ..RenderConfig::default()
    };
    let warm_camera = prepare_camera(
        &ViewConfig::named(NamedView::Iso, CameraKind::Perspective),
        &scene.bounds,
        warm_render.resolution,
    );
    renderer.render_views(
        &scene,
        &[warm_camera],
        &warm_render,
        &LightingConfig::default(),
    )?;
    let setup_ms = setup_started.elapsed().as_secs_f64() * 1_000.0;

    let single_render = RenderConfig {
        resolution: [512, 512],
        material_mode: MaterialMode::Source,
        ..RenderConfig::default()
    };
    let single_cameras = [prepare_camera(
        &ViewConfig::named(NamedView::Front, CameraKind::Orthographic),
        &scene.bounds,
        single_render.resolution,
    )];
    let single = run_case(
        &mut renderer,
        &scene,
        &single_cameras,
        &single_render,
        iterations,
        &output_dir.join("single.png"),
    )?;

    let atlas_render = RenderConfig {
        atlas_columns: Some(2),
        ..single_render
    };
    let atlas_cameras = [
        NamedView::Front,
        NamedView::Right,
        NamedView::Top,
        NamedView::Iso,
    ]
    .map(|view| {
        prepare_camera(
            &ViewConfig::named(view, CameraKind::Orthographic),
            &scene.bounds,
            atlas_render.resolution,
        )
    });
    let atlas = run_case(
        &mut renderer,
        &scene,
        &atlas_cameras,
        &atlas_render,
        iterations,
        &output_dir.join("atlas.png"),
    )?;

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "classification": "in-process GPU-resident scene; wall clock includes render, readback, PNG encode, and file replace",
            "source": source,
            "renderer": renderer.fingerprint(),
            "iterations": iterations,
            "setup_ms": setup_ms,
            "compile_timings_ms": compile_timings,
            "single_512": single,
            "atlas_4x512": atlas,
        }))?
    );
    Ok(())
}

fn run_case(
    renderer: &mut WgpuRenderer,
    scene: &look::scene::CompiledScene,
    cameras: &[look::camera::PreparedCamera],
    render: &RenderConfig,
    iterations: usize,
    output: &PathBuf,
) -> anyhow::Result<serde_json::Value> {
    let mut samples = Vec::with_capacity(iterations);
    let mut gpu_samples = Vec::with_capacity(iterations);
    let mut final_timings = Timings::default();
    for _ in 0..iterations {
        let started = Instant::now();
        let mut batch =
            renderer.render_views(scene, cameras, render, &LightingConfig::default())?;
        if let Some(gpu_render) = batch.timings.get("gpu_render") {
            gpu_samples.push(gpu_render);
        }
        let image = batch.images.pop().context("renderer returned no image")?;
        write_png(output, &image)?;
        samples.push(started.elapsed().as_secs_f64() * 1_000.0);
        final_timings = batch.timings;
    }
    let mut ordered = samples.clone();
    ordered.sort_by(f64::total_cmp);
    let median = ordered[ordered.len() / 2];
    let p95_index = ((ordered.len() - 1) as f64 * 0.95).round() as usize;
    Ok(json!({
        "samples_ms": samples,
        "median_ms": median,
        "p95_ms": ordered[p95_index],
        "min_ms": ordered[0],
        "max_ms": ordered[ordered.len() - 1],
        "gpu_render": distribution(&gpu_samples),
        "final_internal_timings_ms": final_timings,
        "output": output,
    }))
}
fn distribution(samples: &[f64]) -> serde_json::Value {
    if samples.is_empty() {
        return serde_json::Value::Null;
    }
    let mut ordered = samples.to_vec();
    ordered.sort_by(f64::total_cmp);
    let percentile = |value: f64| ordered[((ordered.len() - 1) as f64 * value).round() as usize];
    json!({
        "samples_ms": samples,
        "min_ms": ordered[0],
        "median_ms": percentile(0.5),
        "p95_ms": percentile(0.95),
        "max_ms": ordered[ordered.len() - 1],
    })
}
