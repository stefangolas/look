use std::{
    collections::HashMap,
    env, fs,
    io::{BufRead, BufReader, BufWriter, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    camera::{PreparedCamera, prepare_camera},
    config::{
        CameraKind, LightingConfig, MaterialMode, NamedView, OutputConfig, RenderConfig, UpAxis,
        ViewConfig,
    },
    output::{output_path, write_png},
    renderer::{Renderer, WgpuRenderer},
    scene::{CompiledScene, compile_scene_for_render, prepare_source_textures},
    timing::Timings,
};

const SERVER_STATE_VERSION: u32 = 1;
const DEFAULT_SERVER_IDLE: Duration = Duration::from_secs(600);
const START_TIMEOUT: Duration = Duration::from_secs(20);
static SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServerState {
    version: u32,
    pid: u32,
    address: SocketAddr,
    token: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Envelope {
    token: String,
    request: Request,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method", content = "params", rename_all = "snake_case")]
enum Request {
    Ping,
    Persist {
        source: PathBuf,
        up_axis: UpAxis,
        material_mode: MaterialMode,
        ttl_seconds: u64,
    },
    Render(Box<RenderRequest>),
    Inspect {
        session_id: String,
    },
    Sessions,
    Close {
        session_id: String,
    },
    Shutdown,
}

#[derive(Debug, Serialize, Deserialize)]
struct RenderRequest {
    session_id: String,
    render: RenderConfig,
    lighting: LightingConfig,
    views: Vec<ViewConfig>,
    output: OutputConfig,
    working_directory: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
struct Response {
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

struct Session {
    id: String,
    source: PathBuf,
    scene: CompiledScene,
    material_mode: MaterialMode,
    created_unix_ms: u128,
    last_used: Instant,
    ttl: Duration,
}

pub fn persist_session(
    source: PathBuf,
    up_axis: UpAxis,
    material_mode: MaterialMode,
    ttl_seconds: u64,
) -> anyhow::Result<Value> {
    call(Request::Persist {
        source,
        up_axis,
        material_mode,
        ttl_seconds,
    })
}

pub fn render_session(
    session_id: String,
    render: RenderConfig,
    lighting: LightingConfig,
    views: Vec<ViewConfig>,
    output: OutputConfig,
) -> anyhow::Result<Value> {
    call(Request::Render(Box::new(RenderRequest {
        session_id,
        render,
        lighting,
        views,
        output,
        working_directory: env::current_dir().context("failed to resolve current directory")?,
    })))
}

pub fn inspect_session(session_id: String) -> anyhow::Result<Value> {
    call(Request::Inspect { session_id })
}

pub fn list_sessions() -> anyhow::Result<Value> {
    call(Request::Sessions)
}

pub fn close_session(session_id: String) -> anyhow::Result<Value> {
    call(Request::Close { session_id })
}

pub fn server_status() -> anyhow::Result<Value> {
    let state = read_state()?;
    send(&state, Request::Ping)
}

pub fn stop_server() -> anyhow::Result<Value> {
    let state = read_state()?;
    send(&state, Request::Shutdown)
}

fn call(request: Request) -> anyhow::Result<Value> {
    let state = ensure_server()?;
    send(&state, request)
}

fn ensure_server() -> anyhow::Result<ServerState> {
    if let Ok(state) = read_state()
        && send(&state, Request::Ping).is_ok()
    {
        return Ok(state);
    }

    let state_path = server_state_path()?;
    if state_path.exists() {
        let _ = fs::remove_file(&state_path);
    }
    let executable = env::current_exe().context("failed to locate look executable")?;
    let mut command = Command::new(executable);
    command
        .arg("__serve")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Keep the daemon out of the invoking console and, when the caller is
        // itself in a CI/job object, prevent that job from treating the warm
        // server as unfinished client work.
        command.creation_flags(0x0900_0000);
    }
    command
        .spawn()
        .context("failed to start look session server")?;

    let started = Instant::now();
    while started.elapsed() < START_TIMEOUT {
        if let Ok(state) = read_state()
            && send(&state, Request::Ping).is_ok()
        {
            return Ok(state);
        }
        thread::sleep(Duration::from_millis(40));
    }
    bail!("look session server did not become ready within 20 seconds")
}

fn send(state: &ServerState, request: Request) -> anyhow::Result<Value> {
    if state.version != SERVER_STATE_VERSION {
        bail!("incompatible look session server state")
    }
    let mut stream = TcpStream::connect_timeout(&state.address, Duration::from_secs(2))
        .context("failed to connect to look session server")?;
    stream.set_read_timeout(Some(Duration::from_secs(120)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;
    {
        let mut writer = BufWriter::new(&stream);
        serde_json::to_writer(
            &mut writer,
            &Envelope {
                token: state.token.clone(),
                request,
            },
        )?;
        writer.write_all(b"\n")?;
        writer.flush()?;
    }
    let mut line = String::new();
    BufReader::new(&mut stream)
        .read_line(&mut line)
        .context("failed to read look session response")?;
    let response: Response =
        serde_json::from_str(&line).context("invalid look session response")?;
    if response.status == "ok" {
        Ok(response.result.unwrap_or(Value::Null))
    } else {
        bail!(
            "{}",
            response
                .error
                .unwrap_or_else(|| "look session request failed".to_owned())
        )
    }
}

pub fn run_server() -> anyhow::Result<()> {
    let listener =
        TcpListener::bind(("127.0.0.1", 0)).context("failed to bind look session server")?;
    listener.set_nonblocking(true)?;
    let address = listener.local_addr()?;
    let token_seed = format!("{}:{}:{}", std::process::id(), unix_millis(), address);
    let state = ServerState {
        version: SERVER_STATE_VERSION,
        pid: std::process::id(),
        address,
        token: blake3::hash(token_seed.as_bytes()).to_hex().to_string(),
    };
    let mut renderer = WgpuRenderer::new()?;
    write_state(&state)?;

    let mut sessions = HashMap::<String, Session>::new();
    let mut last_activity = Instant::now();
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                last_activity = Instant::now();
                if serve_connection(stream, &state, &mut renderer, &mut sessions) {
                    break;
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                sessions.retain(|_, session| session.last_used.elapsed() < session.ttl);
                if sessions.is_empty() && last_activity.elapsed() >= DEFAULT_SERVER_IDLE {
                    break;
                }
                thread::sleep(Duration::from_millis(20));
            }
            Err(error) => return Err(error).context("look session server accept failed"),
        }
    }
    remove_own_state(&state);
    Ok(())
}

fn serve_connection(
    mut stream: TcpStream,
    state: &ServerState,
    renderer: &mut WgpuRenderer,
    sessions: &mut HashMap<String, Session>,
) -> bool {
    let mut shutdown = false;
    let result = (|| -> anyhow::Result<Value> {
        stream.set_read_timeout(Some(Duration::from_secs(10)))?;
        let mut line = String::new();
        BufReader::new(&mut stream)
            .take(1024 * 1024)
            .read_line(&mut line)?;
        let envelope: Envelope = serde_json::from_str(&line).context("invalid session request")?;
        if envelope.token != state.token {
            bail!("invalid session server token")
        }
        shutdown = matches!(envelope.request, Request::Shutdown);
        handle_request(envelope.request, renderer, sessions)
    })();
    let response = match result {
        Ok(result) => Response {
            status: "ok".to_owned(),
            result: Some(result),
            error: None,
        },
        Err(error) => Response {
            status: "error".to_owned(),
            result: None,
            error: Some(format!("{error:#}")),
        },
    };
    let mut writer = BufWriter::new(&mut stream);
    let _ = serde_json::to_writer(&mut writer, &response);
    let _ = writer.write_all(b"\n");
    let _ = writer.flush();
    shutdown
}

fn handle_request(
    request: Request,
    renderer: &mut WgpuRenderer,
    sessions: &mut HashMap<String, Session>,
) -> anyhow::Result<Value> {
    match request {
        Request::Ping => Ok(json!({ "server": "look", "pid": std::process::id() })),
        Request::Persist {
            source,
            up_axis,
            material_mode,
            ttl_seconds,
        } => persist_on_server(
            renderer,
            sessions,
            source,
            up_axis,
            material_mode,
            ttl_seconds,
        ),
        Request::Render(params) => {
            let RenderRequest {
                session_id,
                mut render,
                lighting,
                views,
                output,
                working_directory,
            } = *params;
            let session = session_mut(sessions, &session_id)?;
            render.material_mode = session.material_mode;
            let output = resolve_output(output, &working_directory);
            render_on_server(renderer, session, render, lighting, views, output)
        }
        Request::Inspect { session_id } => {
            let session = session_mut(sessions, &session_id)?;
            Ok(session_json(session))
        }
        Request::Sessions => {
            let values = sessions.values().map(session_json).collect::<Vec<_>>();
            Ok(json!({ "sessions": values }))
        }
        Request::Close { session_id } => {
            let removed = sessions
                .remove(&session_id)
                .with_context(|| format!("unknown or expired session '{session_id}'"))?;
            Ok(json!({ "session_id": removed.id, "closed": true }))
        }
        Request::Shutdown => Ok(json!({ "stopped": true, "pid": std::process::id() })),
    }
}

fn persist_on_server(
    renderer: &mut WgpuRenderer,
    sessions: &mut HashMap<String, Session>,
    source: PathBuf,
    up_axis: UpAxis,
    material_mode: MaterialMode,
    ttl_seconds: u64,
) -> anyhow::Result<Value> {
    if ttl_seconds == 0 {
        bail!("session TTL must be positive")
    }
    let started = Instant::now();
    let source = fs::canonicalize(&source)
        .with_context(|| format!("failed to resolve scene '{}'", source.display()))?;
    let mut timings = Timings::default();
    let mut scene = compile_scene_for_render(&source, up_axis, material_mode, &mut timings)?;
    if material_mode == MaterialMode::Source {
        prepare_source_textures(&mut scene, &mut timings)?;
    }

    // Warm the requested shader path and upload the scene before returning the
    // ID, so the first decoupled render is genuinely a hot session render.
    let warm_render = RenderConfig {
        resolution: [1, 1],
        material_mode,
        ..RenderConfig::default()
    };
    let warm_view = ViewConfig::named(NamedView::Iso, CameraKind::Perspective);
    let warm_camera = prepare_camera(&warm_view, &scene.bounds, warm_render.resolution);
    let batch = renderer.render_views(
        &scene,
        &[warm_camera],
        &warm_render,
        &LightingConfig::default(),
    )?;
    timings.merge(&batch.timings);
    timings.record("persist_ready", started.elapsed());

    let counter = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    let id_seed = format!("{}:{}:{}", scene.source_hash, unix_millis(), counter);
    let id = format!(
        "ses_{}",
        &blake3::hash(id_seed.as_bytes()).to_hex().as_str()[..24]
    );
    let session = Session {
        id: id.clone(),
        source,
        scene,
        material_mode,
        created_unix_ms: unix_millis(),
        last_used: Instant::now(),
        ttl: Duration::from_secs(ttl_seconds),
    };
    let result = json!({
        "session": session_json(&session),
        "renderer": renderer.fingerprint(),
        "server_initialization_ms": renderer.initialization_timings(),
        "timings_ms": timings,
    });
    sessions.insert(id, session);
    Ok(result)
}

fn render_on_server(
    renderer: &mut WgpuRenderer,
    session: &mut Session,
    render: RenderConfig,
    lighting: LightingConfig,
    views: Vec<ViewConfig>,
    output: OutputConfig,
) -> anyhow::Result<Value> {
    if views.is_empty() {
        bail!("session render requires at least one view")
    }
    let started = Instant::now();
    let mut timings = Timings::default();
    let cameras = timings.measure("camera", || {
        views
            .iter()
            .map(|view| prepare_camera(view, &session.scene.bounds, render.resolution))
            .collect::<Vec<PreparedCamera>>()
    });
    let mut batch = renderer.render_views(&session.scene, &cameras, &render, &lighting)?;
    timings.merge(&batch.timings);
    let encode_started = Instant::now();
    let view_count = batch.images.len();
    let jobs = batch
        .images
        .drain(..)
        .map(|image| {
            let path = output_path(&output, &image.view, view_count);
            (image, path)
        })
        .collect::<Vec<_>>();
    let outputs = if jobs.len() <= 1 {
        jobs.into_iter()
            .map(|(image, path)| {
                write_png(&path, &image)?;
                Ok(json!({
                    "view": image.view,
                    "path": path,
                    "width": image.width,
                    "height": image.height,
                    "tiles": image.tiles,
                }))
            })
            .collect::<anyhow::Result<Vec<_>>>()?
    } else {
        thread::scope(|scope| {
            jobs.into_iter()
                .map(|(image, path)| {
                    scope.spawn(move || -> anyhow::Result<Value> {
                        write_png(&path, &image)?;
                        Ok(json!({
                            "view": image.view,
                            "path": path,
                            "width": image.width,
                            "height": image.height,
                            "tiles": image.tiles,
                        }))
                    })
                })
                .collect::<Vec<_>>()
                .into_iter()
                .map(|handle| handle.join().expect("PNG encoder thread panicked"))
                .collect::<anyhow::Result<Vec<_>>>()
        })?
    };
    timings.record("png_encode_write", encode_started.elapsed());
    timings.record("session_request_total", started.elapsed());
    session.last_used = Instant::now();
    Ok(json!({
        "session_id": session.id,
        "source_hash": session.scene.source_hash,
        "renderer": renderer.fingerprint(),
        "outputs": outputs,
        "timings_ms": timings,
    }))
}

fn session_mut<'a>(
    sessions: &'a mut HashMap<String, Session>,
    id: &str,
) -> anyhow::Result<&'a mut Session> {
    let session = sessions
        .get_mut(id)
        .with_context(|| format!("unknown or expired session '{id}'"))?;
    if session.last_used.elapsed() >= session.ttl {
        bail!("session '{id}' has expired")
    }
    session.last_used = Instant::now();
    Ok(session)
}

fn session_json(session: &Session) -> Value {
    json!({
        "session_id": session.id,
        "source": session.source,
        "source_hash": session.scene.source_hash,
        "material_mode": session.material_mode,
        "created_unix_ms": session.created_unix_ms,
        "expires_in_s": session.ttl.saturating_sub(session.last_used.elapsed()).as_secs(),
        "statistics": session.scene.statistics,
    })
}

fn resolve_output(mut output: OutputConfig, working_directory: &Path) -> OutputConfig {
    if output.directory.is_relative() {
        output.directory = working_directory.join(output.directory);
    }
    if let Some(path) = output.single_file.as_mut()
        && path.is_relative()
    {
        *path = working_directory.join(&*path);
    }
    output
}

fn read_state() -> anyhow::Result<ServerState> {
    let bytes = fs::read(server_state_path()?).context("look session server is not running")?;
    serde_json::from_slice(&bytes).context("invalid look session server state")
}

fn write_state(state: &ServerState) -> anyhow::Result<()> {
    let path = server_state_path()?;
    let directory = path.parent().context("invalid server state path")?;
    fs::create_dir_all(directory)?;
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    let bytes = serde_json::to_vec(state)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(&temporary)?;
        file.write_all(&bytes)?;
    }
    #[cfg(not(unix))]
    fs::write(&temporary, bytes)?;
    if path.exists() {
        fs::remove_file(&path)?;
    }
    fs::rename(temporary, path)?;
    Ok(())
}

fn remove_own_state(state: &ServerState) {
    let Ok(path) = server_state_path() else {
        return;
    };
    if let Ok(current) = read_state()
        && current.token == state.token
    {
        let _ = fs::remove_file(path);
    }
}

pub fn server_state_path() -> anyhow::Result<PathBuf> {
    let root = if let Some(path) = env::var_os("LOOK_CACHE_DIR") {
        PathBuf::from(path)
    } else {
        platform_cache_root()
            .context("could not determine the platform cache directory")?
            .join("look")
            .join("cache")
    };
    Ok(root.join("server.json"))
}

#[cfg(target_os = "windows")]
fn platform_cache_root() -> Option<PathBuf> {
    env::var_os("LOCALAPPDATA").map(PathBuf::from)
}

#[cfg(not(target_os = "windows"))]
fn platform_cache_root() -> Option<PathBuf> {
    env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
}

fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
