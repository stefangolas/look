use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};

pub const CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum CameraKind {
    Perspective,
    Orthographic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum MaterialMode {
    Technical,
    Source,
}

/// Named lighting/camera compatibility profiles. `F3dMatch` is pinned to the
/// default vtkLightKit values used by F3D 3.5 rather than tracking an
/// unspecified future F3D release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum LightingPreset {
    Technical,
    F3dMatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NamedView {
    Front,
    Rear,
    Left,
    Right,
    Top,
    Bottom,
    Iso,
}

impl NamedView {
    pub fn id(self) -> &'static str {
        match self {
            Self::Front => "front",
            Self::Rear => "rear",
            Self::Left => "left",
            Self::Right => "right",
            Self::Top => "top",
            Self::Bottom => "bottom",
            Self::Iso => "iso",
        }
    }

    /// Direction from the target toward the camera in normalized Y-up space.
    pub fn direction(self) -> [f32; 3] {
        match self {
            Self::Front => [0.0, 0.0, 1.0],
            Self::Rear => [0.0, 0.0, -1.0],
            Self::Left => [-1.0, 0.0, 0.0],
            Self::Right => [1.0, 0.0, 0.0],
            Self::Top => [0.0, 1.0, 0.0],
            Self::Bottom => [0.0, -1.0, 0.0],
            Self::Iso => [1.0, 1.0, 1.0],
        }
    }
}

impl std::str::FromStr for NamedView {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "front" => Ok(Self::Front),
            "rear" | "back" => Ok(Self::Rear),
            "left" => Ok(Self::Left),
            "right" => Ok(Self::Right),
            "top" => Ok(Self::Top),
            "bottom" => Ok(Self::Bottom),
            "iso" | "isometric" => Ok(Self::Iso),
            _ => Err(format!("unknown view '{value}'")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum UpAxis {
    X,
    Y,
    Z,
}

#[derive(Debug, Clone, Serialize)]
pub struct NormalizedConfig {
    pub scene: SceneConfig,
    pub render: RenderConfig,
    pub lighting: LightingConfig,
    pub views: Vec<ViewConfig>,
    pub output: OutputConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneConfig {
    pub source: PathBuf,
    #[serde(default = "default_up_axis")]
    pub up_axis: UpAxis,
    #[serde(default)]
    pub units: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderConfig {
    #[serde(default = "default_resolution")]
    pub resolution: [u32; 2],
    #[serde(default = "default_background")]
    pub background: String,
    #[serde(default = "default_base_color")]
    pub base_color: String,
    #[serde(default = "default_material_mode")]
    pub material_mode: MaterialMode,
    #[serde(default)]
    pub antialias: bool,
    /// When set, pack all views into one GPU-rendered PNG atlas using this
    /// many tile columns. Resolution remains the per-view tile size.
    #[serde(default)]
    pub atlas_columns: Option<u32>,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            resolution: default_resolution(),
            background: default_background(),
            base_color: default_base_color(),
            material_mode: default_material_mode(),
            antialias: false,
            atlas_columns: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LightingConfig {
    #[serde(default = "default_lighting_preset")]
    pub preset: LightingPreset,
    #[serde(default = "default_ambient")]
    pub ambient: f32,
    #[serde(default = "default_light_direction")]
    pub direction: [f32; 3],
    #[serde(default = "default_light_intensity")]
    pub intensity: f32,
    #[serde(default = "default_light_color")]
    pub color: String,
}

impl Default for LightingConfig {
    fn default() -> Self {
        Self {
            preset: default_lighting_preset(),
            ambient: default_ambient(),
            direction: default_light_direction(),
            intensity: default_light_intensity(),
            color: default_light_color(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewConfig {
    pub id: String,
    #[serde(default = "default_camera_kind", rename = "type")]
    pub kind: CameraKind,
    pub direction: [f32; 3],
    #[serde(default)]
    pub up: Option<[f32; 3]>,
    #[serde(default = "default_fov")]
    pub fov_degrees: f32,
    #[serde(default = "default_padding")]
    pub padding: f32,
}

impl ViewConfig {
    pub fn named(view: NamedView, kind: CameraKind) -> Self {
        Self {
            id: view.id().to_owned(),
            kind,
            direction: view.direction(),
            up: None,
            fov_degrees: default_fov(),
            padding: default_padding(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputConfig {
    #[serde(default = "default_output_directory")]
    pub directory: PathBuf,
    #[serde(default = "default_naming")]
    pub naming: String,
    #[serde(default)]
    pub single_file: Option<PathBuf>,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            directory: default_output_directory(),
            naming: default_naming(),
            single_file: None,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct JobFile {
    version: u32,
    scene: SceneConfig,
    #[serde(default)]
    render: RenderConfig,
    #[serde(default)]
    lighting: LightingConfig,
    views: Vec<ViewConfig>,
    #[serde(default)]
    output: OutputConfig,
}

impl NormalizedConfig {
    pub fn from_yaml(path: &Path) -> anyhow::Result<Self> {
        let bytes = fs::read(path)
            .with_context(|| format!("failed to read configuration '{}'", path.display()))?;
        let mut job: JobFile = serde_yaml::from_slice(&bytes)
            .with_context(|| format!("invalid YAML configuration '{}'", path.display()))?;
        if job.version != CONFIG_VERSION {
            bail!(
                "unsupported configuration version {}; expected {}",
                job.version,
                CONFIG_VERSION
            );
        }
        if job.scene.source.is_relative() {
            let base = path.parent().unwrap_or_else(|| Path::new("."));
            job.scene.source = base.join(&job.scene.source);
        }
        let config = Self {
            scene: job.scene,
            render: job.render,
            lighting: job.lighting,
            views: job.views,
            output: job.output,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.views.is_empty() {
            bail!("at least one view is required");
        }
        if self.render.resolution[0] == 0 || self.render.resolution[1] == 0 {
            bail!("render resolution must be non-zero");
        }
        if self.render.resolution[0] > 16_384 || self.render.resolution[1] > 16_384 {
            bail!("render resolution exceeds the 16384 pixel safety limit");
        }
        if self.render.atlas_columns == Some(0) {
            bail!("atlas column count must be positive");
        }
        if self
            .views
            .iter()
            .any(|view| !(1.0..179.0).contains(&view.fov_degrees))
        {
            bail!("camera field of view must be between 1 and 179 degrees");
        }
        if self.views.iter().any(|view| view.padding <= 0.0) {
            bail!("camera padding must be positive");
        }
        parse_hex_color(&self.render.background)?;
        parse_hex_color(&self.render.base_color)?;
        parse_hex_color(&self.lighting.color)?;
        Ok(())
    }
}

pub fn parse_resolution(value: &str) -> Result<[u32; 2], String> {
    let (width, height) = value
        .split_once(['x', 'X'])
        .ok_or_else(|| "resolution must look like WIDTHxHEIGHT".to_owned())?;
    let width = width
        .parse::<u32>()
        .map_err(|_| "invalid resolution width".to_owned())?;
    let height = height
        .parse::<u32>()
        .map_err(|_| "invalid resolution height".to_owned())?;
    if width == 0 || height == 0 {
        return Err("resolution must be non-zero".to_owned());
    }
    Ok([width, height])
}

pub fn parse_vec3(value: &str) -> Result<[f32; 3], String> {
    let components = value
        .split(',')
        .map(str::trim)
        .map(|item| item.parse::<f32>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "vector must contain three comma-separated numbers".to_owned())?;
    components
        .try_into()
        .map_err(|_| "vector must contain exactly three components".to_owned())
}

pub fn parse_hex_color(value: &str) -> anyhow::Result<[f32; 4]> {
    let value = value.strip_prefix('#').unwrap_or(value);
    if value.len() != 6 {
        bail!("color must use #RRGGBB format");
    }
    let channel = |range| -> anyhow::Result<f32> {
        let byte = u8::from_str_radix(&value[range], 16).context("invalid hexadecimal color")?;
        Ok(srgb_to_linear(f32::from(byte) / 255.0))
    };
    Ok([channel(0..2)?, channel(2..4)?, channel(4..6)?, 1.0])
}

fn srgb_to_linear(value: f32) -> f32 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn default_up_axis() -> UpAxis {
    UpAxis::Y
}
fn default_resolution() -> [u32; 2] {
    [1024, 1024]
}
fn default_background() -> String {
    "#252525".to_owned()
}
fn default_base_color() -> String {
    "#b8c0c8".to_owned()
}
fn default_material_mode() -> MaterialMode {
    MaterialMode::Technical
}
fn default_ambient() -> f32 {
    0.35
}
fn default_lighting_preset() -> LightingPreset {
    LightingPreset::Technical
}
fn default_light_direction() -> [f32; 3] {
    [-1.0, -2.0, -3.0]
}
fn default_light_intensity() -> f32 {
    0.85
}
fn default_light_color() -> String {
    "#ffffff".to_owned()
}
fn default_camera_kind() -> CameraKind {
    CameraKind::Perspective
}
fn default_fov() -> f32 {
    35.0
}
fn default_padding() -> f32 {
    1.1
}
fn default_output_directory() -> PathBuf {
    PathBuf::from("renders")
}
fn default_naming() -> String {
    "{view}.png".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_resolution() {
        assert_eq!(parse_resolution("640x480").unwrap(), [640, 480]);
        assert!(parse_resolution("640").is_err());
    }

    #[test]
    fn named_views_have_stable_ids() {
        assert_eq!(NamedView::Iso.id(), "iso");
        assert_eq!("back".parse::<NamedView>().unwrap(), NamedView::Rear);
    }

    #[test]
    fn color_is_converted_to_linear() {
        let white = parse_hex_color("#ffffff").unwrap();
        let black = parse_hex_color("#000000").unwrap();
        assert_eq!(white, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(black, [0.0, 0.0, 0.0, 1.0]);
    }
}
