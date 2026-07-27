use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::Context;
use clap::{Args, Parser, Subcommand};
use serde::Serialize;

use crate::{
    cache::MetadataCache,
    camera::{PreparedCamera, prepare_camera},
    config::{
        CameraKind, LightingConfig, MaterialMode, NamedView, NormalizedConfig, OutputConfig,
        RenderConfig, SceneConfig, UpAxis, ViewConfig, parse_resolution, parse_vec3,
    },
    output::{output_path, write_png},
    renderer::{HardwareFingerprint, Renderer, WgpuRenderer},
    scene::{SceneStatistics, compile_glb, compile_glb_for_render, prepare_source_textures},
    timing::Timings,
};

#[derive(Debug, Parser)]
#[command(name = "v3", version, about, arg_required_else_help = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

impl Cli {
    pub fn parse_normalized() -> Self {
        let mut arguments = std::env::args_os().collect::<Vec<_>>();
        if should_imply_render(&arguments) {
            arguments.insert(1, OsString::from("render"));
        }
        Self::parse_from(arguments)
    }
}

fn should_imply_render(arguments: &[OsString]) -> bool {
    let Some(first) = arguments.get(1).and_then(|value| value.to_str()) else {
        return false;
    };
    !first.starts_with('-')
        && !matches!(
            first,
            "render" | "run" | "inspect" | "doctor" | "help" | "version"
        )
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Render one or more technical views of a GLB.
    Render(RenderArgs),
    /// Execute a declarative YAML render job.
    Run(RunArgs),
    /// Inspect geometry and scene statistics without initializing the GPU.
    Inspect(InspectArgs),
    /// Report the selected GPU backend and adapter.
    Doctor(DoctorArgs),
}

#[derive(Debug, Args)]
pub struct RenderArgs {
    pub scene: PathBuf,

    #[arg(
        long = "views",
        alias = "view",
        value_delimiter = ',',
        default_value = "iso"
    )]
    pub views: Vec<NamedView>,

    #[arg(long, value_enum, default_value = "perspective")]
    pub camera: CameraKind,

    #[arg(long, value_parser = parse_resolution, default_value = "1024x1024")]
    pub resolution: [u32; 2],

    #[arg(long, default_value = "#252525")]
    pub background: String,

    #[arg(long, default_value = "#b8c0c8")]
    pub base_color: String,

    #[arg(long, value_enum, default_value = "technical")]
    pub material_mode: MaterialMode,

    #[arg(long, default_value_t = 0.35)]
    pub ambient: f32,

    #[arg(long, value_parser = parse_vec3, default_value = "-1,-2,-3", allow_hyphen_values = true)]
    pub light_direction: [f32; 3],

    #[arg(long, default_value_t = 0.85)]
    pub light_intensity: f32,

    #[arg(long, default_value = "#ffffff")]
    pub light_color: String,

    #[arg(long)]
    pub antialias: bool,

    #[arg(long)]
    pub output: Option<PathBuf>,

    #[arg(long, default_value = "renders")]
    pub output_dir: PathBuf,

    #[arg(long, value_enum, default_value = "y")]
    pub up_axis: UpAxis,

    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct RunArgs {
    pub config: PathBuf,

    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct InspectArgs {
    pub scene: PathBuf,

    #[arg(long, value_enum, default_value = "y")]
    pub up_axis: UpAxis,

    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct DoctorArgs {
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Serialize)]
struct CommandResult<'a> {
    status: &'static str,
    scene: SceneResult<'a>,
    renderer: &'a HardwareFingerprint,
    outputs: Vec<OutputResult>,
    timings_ms: Timings,
}

#[derive(Debug, Serialize)]
struct SceneResult<'a> {
    source: &'a Path,
    hash: &'a str,
    statistics: &'a SceneStatistics,
}

#[derive(Debug, Serialize)]
struct OutputResult {
    view: String,
    path: PathBuf,
    width: u32,
    height: u32,
}

pub fn execute_render(args: RenderArgs) -> anyhow::Result<()> {
    let output = OutputConfig {
        directory: args.output_dir,
        naming: "{view}.png".to_owned(),
        single_file: args.output,
    };
    let config = NormalizedConfig {
        scene: SceneConfig {
            source: args.scene,
            up_axis: args.up_axis,
            units: None,
        },
        render: RenderConfig {
            resolution: args.resolution,
            background: args.background,
            base_color: args.base_color,
            material_mode: args.material_mode,
            antialias: args.antialias,
        },
        lighting: LightingConfig {
            ambient: args.ambient,
            direction: args.light_direction,
            intensity: args.light_intensity,
            color: args.light_color,
        },
        views: args
            .views
            .into_iter()
            .map(|view| ViewConfig::named(view, args.camera))
            .collect(),
        output,
    };
    config.validate()?;
    execute_config(config, args.json)
}

pub fn execute_job(args: RunArgs) -> anyhow::Result<()> {
    let config = NormalizedConfig::from_yaml(&args.config)?;
    execute_config(config, args.json)
}

pub fn execute_inspect(args: InspectArgs) -> anyhow::Result<()> {
    require_glb(&args.scene)?;
    let mut timings = Timings::default();
    let cache = MetadataCache::platform_default();
    let cache_started = Instant::now();
    let cached = cache
        .as_ref()
        .and_then(|cache| cache.load(&args.scene, args.up_axis).ok().flatten());
    timings.record("cache_lookup", cache_started.elapsed());
    if let Some(cached) = cached {
        if args.json {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": "ok",
                    "cache": "hit",
                    "source": args.scene,
                    "hash": cached.source_hash,
                    "statistics": cached.statistics,
                    "timings_ms": timings,
                }))?
            );
        } else {
            print_inspection(&args.scene, &cached.source_hash, &cached.statistics, true);
        }
        return Ok(());
    }
    let scene = compile_glb(&args.scene, args.up_axis, &mut timings)?;
    if let Some(cache) = cache {
        let _ = cache.store(
            &args.scene,
            args.up_axis,
            &scene.source_hash,
            &scene.statistics,
        );
    }
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "status": "ok",
                "cache": "miss",
                "source": args.scene,
                "hash": scene.source_hash,
                "statistics": scene.statistics,
                "timings_ms": timings,
            }))?
        );
    } else {
        print_inspection(&args.scene, &scene.source_hash, &scene.statistics, false);
    }
    Ok(())
}

pub fn execute_doctor(args: DoctorArgs) -> anyhow::Result<()> {
    let renderer = WgpuRenderer::new()?;
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "status": "ok",
                "renderer": renderer.fingerprint(),
                "timings_ms": renderer.initialization_timings(),
            }))?
        );
    } else {
        let fingerprint = renderer.fingerprint();
        println!("backend: {}", fingerprint.backend);
        println!("adapter: {}", fingerprint.adapter);
        println!("driver: {} {}", fingerprint.driver, fingerprint.driver_info);
    }
    Ok(())
}

fn execute_config(config: NormalizedConfig, json: bool) -> anyhow::Result<()> {
    require_glb(&config.scene.source)?;
    let total_started = Instant::now();
    let renderer_thread = std::thread::spawn(WgpuRenderer::new);
    let mut timings = Timings::default();
    let mut scene = compile_glb_for_render(
        &config.scene.source,
        config.scene.up_axis,
        config.render.material_mode,
        &mut timings,
    )?;
    if config.render.material_mode == MaterialMode::Source {
        prepare_source_textures(&mut scene, &mut timings)?;
    }
    if let Some(cache) = MetadataCache::platform_default() {
        let cache_started = Instant::now();
        let _ = cache.store(
            &config.scene.source,
            config.scene.up_axis,
            &scene.source_hash,
            &scene.statistics,
        );
        timings.record("cache_store", cache_started.elapsed());
    }
    let cameras = timings.measure("camera", || {
        config
            .views
            .iter()
            .map(|view| prepare_camera(view, &scene.bounds, config.render.resolution))
            .collect::<Vec<PreparedCamera>>()
    });

    let join_started = Instant::now();
    let mut renderer = renderer_thread
        .join()
        .map_err(|_| anyhow::anyhow!("GPU initialization thread panicked"))??;
    timings.record("gpu_init_join_wait", join_started.elapsed());
    let mut batch = renderer.render_views(&scene, &cameras, &config.render, &config.lighting)?;
    timings.merge(&batch.timings);

    let encode_started = Instant::now();
    let view_count = batch.images.len();
    let jobs = batch
        .images
        .drain(..)
        .map(|image| {
            let path = output_path(&config.output, &image.view, view_count);
            (image, path)
        })
        .collect::<Vec<_>>();
    let outputs = if jobs.len() <= 1 {
        jobs.into_iter()
            .map(|(image, path)| encode_output(image, path))
            .collect::<anyhow::Result<Vec<_>>>()?
    } else {
        std::thread::scope(|scope| {
            jobs.into_iter()
                .map(|(image, path)| scope.spawn(move || encode_output(image, path)))
                .collect::<Vec<_>>()
                .into_iter()
                .map(|handle| handle.join().expect("PNG encoder thread panicked"))
                .collect::<anyhow::Result<Vec<_>>>()
        })?
    };
    timings.record("png_encode_write", encode_started.elapsed());
    timings.record("total", total_started.elapsed());

    let result = CommandResult {
        status: "ok",
        scene: SceneResult {
            source: &config.scene.source,
            hash: &scene.source_hash,
            statistics: &scene.statistics,
        },
        renderer: renderer.fingerprint(),
        outputs,
        timings_ms: timings,
    };
    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        for output in &result.outputs {
            println!("{}: {}", output.view, output.path.display());
        }
        println!(
            "{} triangles, {} instances on {} ({})",
            scene.statistics.triangles,
            scene.statistics.instances,
            result.renderer.adapter,
            result.renderer.backend
        );
    }
    Ok(())
}

fn encode_output(
    image: crate::renderer::RenderedImage,
    path: PathBuf,
) -> anyhow::Result<OutputResult> {
    write_png(&path, &image)?;
    Ok(OutputResult {
        view: image.view,
        path,
        width: image.width,
        height: image.height,
    })
}

fn require_glb(path: &Path) -> anyhow::Result<()> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if !extension.eq_ignore_ascii_case("glb") {
        anyhow::bail!(
            "unsupported scene '{}'; the MVP currently accepts GLB files only",
            path.display()
        );
    }
    if !path.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("scene '{}' does not exist", path.display()),
        ))
        .with_context(|| "scene validation failed");
    }
    Ok(())
}

fn print_inspection(source: &Path, source_hash: &str, statistics: &SceneStatistics, cached: bool) {
    println!("source: {}", source.display());
    println!("hash: {source_hash}");
    println!("cache: {}", if cached { "hit" } else { "miss" });
    println!("nodes: {}", statistics.nodes);
    println!("mesh primitives: {}", statistics.mesh_primitives);
    println!("unique geometries: {}", statistics.unique_geometries);
    println!("instances: {}", statistics.instances);
    println!("triangles: {}", statistics.triangles);
    println!(
        "bounds: {:?} to {:?}",
        statistics.bounds.min, statistics.bounds.max
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_path_implies_render_command() {
        let args = vec![OsString::from("v3"), OsString::from("part.glb")];
        assert!(should_imply_render(&args));
    }

    #[test]
    fn explicit_commands_are_not_rewritten() {
        let args = vec![OsString::from("v3"), OsString::from("inspect")];
        assert!(!should_imply_render(&args));
    }
}
