use std::fs::{self, OpenOptions};
use std::io::Write;
use std::net::TcpListener as StdTcpListener;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, RwLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::extract::{DefaultBodyLimit, Path as AxumPath, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::routing::get;
use axum::{extract::Request, response::IntoResponse};
use axum::{Json, Router};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use tokio::sync::oneshot;
use zeroize::Zeroizing;

use crate::layout::LayoutMode;
use crate::session::actions::{Action, ActionError, Dispatcher, PresetInfo, Snapshot, Target};

const CONTROL_DIR: &str = "control";
const HELPER_DIR: &str = "helpers";
const MAX_BODY_BYTES: usize = 1024 * 1024;
const MAX_CONCURRENT_REQUESTS: usize = 32;
const MAX_PAYLOAD_BYTES: usize = 64 * 1024;
const MAX_READ_CHARS: usize = 200_000;
const DEFAULT_READ_CHARS: usize = 8_000;
const DEFAULT_RUN_CHARS: usize = 12_000;
const DEFAULT_RUN_QUIET_MS: u64 = 1_200;
const DEFAULT_RUN_MAX_WAIT_MS: u64 = 8_000;
const DEFAULT_RUN_INTERVAL_MS: u64 = 250;

struct ApiState {
    token: Zeroizing<String>,
    base_url: String,
    info_path: String,
    instance_id: String,
    snapshot: Arc<RwLock<Snapshot>>,
    dispatcher: Dispatcher,
    repaint: egui::Context,
    request_slots: Arc<tokio::sync::Semaphore>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusResponse {
    ok: bool,
    base_url: String,
    info_path: String,
    connected_tabs: usize,
    instance_id: String,
    window_label: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TabsResponse {
    tabs: Vec<TabSummary>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TabSummary {
    tab_id: String,
    pty_id: u64,
    title: String,
    shell: String,
    is_active: bool,
    stored_output_chars: usize,
    output_truncated: bool,
    last_input: String,
    last_updated_at_ms: u64,
    last_input_at_ms: Option<u64>,
    last_output_at_ms: Option<u64>,
    output_sequence: u64,
    ready: bool,
    exited: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DiscoveryFile {
    base_url: String,
    auth_token: String,
    updated_at_ms: u64,
    instance_id: String,
    schema_version: u32,
    capabilities: [&'static str; 11],
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateRequest {
    name: String,
    shell: Option<String>,
    cwd: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RenameRequest {
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenLayoutRequest {
    names: Vec<String>,
    layout: String,
    columns: Option<usize>,
    shell: Option<String>,
    cwd: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PresetRunRequest {
    label: String,
    tab: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PresetSummary {
    kind: String,
    label: String,
    command: String,
    send_enter: bool,
}

#[derive(Serialize)]
struct PresetsResponse {
    presets: Vec<PresetSummary>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LayoutResponse {
    ok: bool,
    detail: String,
    layout: String,
    columns: Option<usize>,
    tabs: Vec<TabSummary>,
    active_tab_id: Option<String>,
    visible_tab_ids: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReadQuery {
    chars: Option<usize>,
    lines: Option<usize>,
    from: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PayloadRequest {
    text: Option<String>,
    payload_base64: Option<String>,
    delivery: Option<String>,
    delay_ms: Option<u64>,
    #[serde(default)]
    enter: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct KeyRequest {
    key: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunRequest {
    text: Option<String>,
    payload_base64: Option<String>,
    delivery: Option<String>,
    delay_ms: Option<u64>,
    #[serde(default = "default_true")]
    enter: bool,
    chars: Option<usize>,
    quiet_ms: Option<u64>,
    max_wait_ms: Option<u64>,
    interval_ms: Option<u64>,
    wait_for_text: Option<String>,
    #[serde(default)]
    ignore_case: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReadResponse {
    tab: TabSummary,
    text: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ActionResponse {
    ok: bool,
    tab: TabSummary,
    detail: String,
    delivery: Option<String>,
    bytes_sent: Option<usize>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RunResponse {
    ok: bool,
    tab: TabSummary,
    detail: String,
    delivery: String,
    bytes_sent: usize,
    text: String,
    chars: usize,
    elapsed_ms: u64,
    timed_out: bool,
    stable: bool,
    completion_reason: String,
    matched_text: Option<String>,
}

pub(crate) struct ControlServer {
    info_path: PathBuf,
    helper_path: PathBuf,
    instance_id: String,
    shutdown: Option<oneshot::Sender<()>>,
    _thread: Option<JoinHandle<()>>,
}

impl ControlServer {
    pub(crate) fn start(
        native_root: &Path,
        snapshot: Arc<RwLock<Snapshot>>,
        dispatcher: Dispatcher,
        repaint: egui::Context,
    ) -> Result<Self, String> {
        let helper_path = install_cli_helper(native_root)?;
        let control_dir = native_root.join(CONTROL_DIR);
        fs::create_dir_all(&control_dir).map_err(|error| error.to_string())?;
        set_private_directory_permissions(&control_dir)?;

        let instance_id = random_hex(16)?;
        let token = random_hex(32)?;
        let info_path = control_dir.join(format!("{instance_id}.json"));
        let listener = StdTcpListener::bind("127.0.0.1:0").map_err(|error| error.to_string())?;
        listener
            .set_nonblocking(true)
            .map_err(|error| error.to_string())?;
        let address = listener.local_addr().map_err(|error| error.to_string())?;
        let base_url = format!("http://{address}");
        let state = Arc::new(ApiState {
            token: Zeroizing::new(token.clone()),
            base_url: base_url.clone(),
            info_path: info_path.to_string_lossy().into_owned(),
            instance_id: instance_id.clone(),
            snapshot,
            dispatcher,
            repaint,
            request_slots: Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_REQUESTS)),
        });
        let router = Router::new()
            .route("/v1/status", get(status))
            .route("/v1/tabs", get(tabs).post(create_tab))
            .route("/v1/tabs/{selector}/read", get(read_tab))
            .route("/v1/tabs/{selector}/send", axum::routing::post(send_tab))
            .route("/v1/tabs/{selector}/key", axum::routing::post(send_key))
            .route("/v1/tabs/{selector}/run", axum::routing::post(run_command))
            .route(
                "/v1/tabs/{selector}/rename",
                axum::routing::post(rename_tab),
            )
            .route("/v1/layout/open", axum::routing::post(open_layout))
            .route("/v1/presets", get(presets))
            .route("/v1/presets/run", axum::routing::post(run_preset))
            .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
            .layer(middleware::from_fn_with_state(
                Arc::clone(&state),
                limit_concurrent_requests,
            ))
            .with_state(state);

        let (shutdown, shutdown_rx) = oneshot::channel();
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name("buttonscli-control-api".into())
            .spawn(move || {
                let runtime = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        let _ = ready_tx.send(Err(error.to_string()));
                        return;
                    }
                };
                runtime.block_on(async move {
                    let listener = match tokio::net::TcpListener::from_std(listener) {
                        Ok(listener) => listener,
                        Err(error) => {
                            let _ = ready_tx.send(Err(error.to_string()));
                            return;
                        }
                    };
                    let _ = ready_tx.send(Ok(()));
                    if let Err(error) = axum::serve(listener, router)
                        .with_graceful_shutdown(async {
                            let _ = shutdown_rx.await;
                        })
                        .await
                    {
                        log::error!("native control API stopped: {error}");
                    }
                });
            })
            .map_err(|error| error.to_string())?;

        match ready_rx.recv_timeout(Duration::from_secs(3)) {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                let _ = shutdown.send(());
                let _ = thread.join();
                return Err(format!("control API could not start: {error}"));
            }
            Err(error) => {
                let _ = shutdown.send(());
                return Err(format!("control API startup timed out: {error}"));
            }
        }

        let descriptor = DiscoveryFile {
            base_url,
            auth_token: token,
            updated_at_ms: now_ms(),
            instance_id: instance_id.clone(),
            schema_version: 1,
            capabilities: [
                "status",
                "tabs",
                "read",
                "send",
                "key",
                "run",
                "create",
                "rename",
                "layout",
                "presets",
                "preset-run",
            ],
        };
        if let Err(error) = write_discovery_file(&info_path, &descriptor) {
            let _ = shutdown.send(());
            let _ = thread.join();
            return Err(error);
        }

        Ok(Self {
            info_path,
            helper_path,
            instance_id,
            shutdown: Some(shutdown),
            _thread: Some(thread),
        })
    }

    pub(crate) fn agent_instructions(&self) -> String {
        let info_path = self.info_path.to_string_lossy();
        let helper_path = self.helper_path.to_string_lossy();
        format!(
            "Use the native ButtonsCLI instance selected by this exact connection file. The file contains a temporary local token; read it through the helper and never copy or print its contents.\n\nPowerShell:\n$env:BUTTONSCLI_CONTROL_INFO_PATH = {}\nnode {} status --json\nnode {} tabs --json\n\nFor other commands, use the same environment variable and helper path. This native instance supports status, tabs, read, send, key, run, create, rename, layout, and presets. Input modes are raw, bracketed, and paced slow-typed. Grid columns respect the current window's minimum pane sizes and may be reduced when space is limited.\n",
            powershell_literal(&info_path),
            powershell_literal(&helper_path),
            powershell_literal(&helper_path),
        )
    }
}

fn install_cli_helper(native_root: &Path) -> Result<PathBuf, String> {
    let helper_source = include_bytes!("../../scripts/buttonsclictl.mjs");
    let digest = Sha256::digest(helper_source);
    let mut name = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(name, "{byte:02x}");
    }
    let helper_dir = native_root.join(HELPER_DIR);
    fs::create_dir_all(&helper_dir).map_err(|error| error.to_string())?;
    set_private_directory_permissions(&helper_dir)?;
    let helper_path = helper_dir.join(format!("buttonsclictl-{name}.mjs"));
    if helper_path.is_file() {
        let current = fs::read(&helper_path).map_err(|error| error.to_string())?;
        if current != helper_source {
            return Err("installed native CLI helper does not match its version hash".into());
        }
        return Ok(helper_path);
    }

    let temp_path = helper_dir.join(format!(".{name}-{}.tmp", std::process::id()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&temp_path)
        .map_err(|error| error.to_string())?;
    if let Err(error) = file.write_all(helper_source).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(&temp_path);
        return Err(error.to_string());
    }
    drop(file);
    if let Err(error) = fs::rename(&temp_path, &helper_path) {
        let _ = fs::remove_file(&temp_path);
        if helper_path.is_file()
            && fs::read(&helper_path).is_ok_and(|current| current == helper_source)
        {
            return Ok(helper_path);
        }
        return Err(error.to_string());
    }
    Ok(helper_path)
}

fn powershell_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

impl Drop for ControlServer {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        if fs::read(&self.info_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
            .and_then(|value| value.get("instanceId")?.as_str().map(str::to_owned))
            .is_some_and(|instance_id| instance_id == self.instance_id)
        {
            if let Err(error) = fs::remove_file(&self.info_path) {
                if error.kind() != std::io::ErrorKind::NotFound {
                    log::warn!("could not remove native control discovery file: {error}");
                }
            }
        }
    }
}

async fn limit_concurrent_requests(
    State(state): State<Arc<ApiState>>,
    request: Request,
    next: Next,
) -> axum::response::Response {
    let Ok(_permit) = Arc::clone(&state.request_slots).try_acquire_owned() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "control API is busy"})),
        )
            .into_response();
    };
    next.run(request).await
}

async fn status(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> Result<Json<StatusResponse>, (StatusCode, Json<serde_json::Value>)> {
    authorize(&headers, &state)?;
    let snapshot = state.snapshot.read().map_err(|_| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "session state unavailable",
        )
    })?;
    Ok(Json(StatusResponse {
        ok: true,
        base_url: state.base_url.clone(),
        info_path: state.info_path.clone(),
        connected_tabs: snapshot.sessions.len(),
        instance_id: state.instance_id.clone(),
        window_label: "main",
    }))
}

async fn tabs(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> Result<Json<TabsResponse>, (StatusCode, Json<serde_json::Value>)> {
    authorize(&headers, &state)?;
    let snapshot = read_snapshot(&state)?;
    let mut sessions = snapshot.sessions.clone();
    sessions.sort_by(|left, right| {
        (snapshot.active_id != Some(left.id))
            .cmp(&(snapshot.active_id != Some(right.id)))
            .then_with(|| left.id.cmp(&right.id))
    });
    let tabs = sessions
        .into_iter()
        .map(|session| tab_summary(&session, snapshot.active_id))
        .collect();
    Ok(Json(TabsResponse { tabs }))
}

async fn create_tab(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Json(request): Json<CreateRequest>,
) -> Result<Json<ActionResponse>, (StatusCode, Json<serde_json::Value>)> {
    authorize(&headers, &state)?;
    let action = Action::CreateNamed {
        name: request.name.clone(),
        shell: request.shell,
        cwd: request.cwd,
    };
    action.validate().map_err(map_action_error)?;
    let snapshot = read_snapshot(&state)?;
    let id = dispatch_action(&state, &snapshot, None, action)
        .await?
        .ok_or_else(|| api_error(StatusCode::INTERNAL_SERVER_ERROR, "created tab has no ID"))?;
    let snapshot = read_snapshot(&state)?;
    let session = find_session(&snapshot, id)?;
    Ok(Json(ActionResponse {
        ok: true,
        tab: tab_summary(session, snapshot.active_id),
        detail: format!("Created tab '{}'.", request.name.trim()),
        delivery: None,
        bytes_sent: None,
    }))
}

async fn rename_tab(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    AxumPath(selector): AxumPath<String>,
    Json(request): Json<RenameRequest>,
) -> Result<Json<ActionResponse>, (StatusCode, Json<serde_json::Value>)> {
    authorize(&headers, &state)?;
    let snapshot = read_snapshot(&state)?;
    let id = snapshot
        .resolve(&Target::Selector(selector))
        .map_err(map_action_error)?;
    let action = Action::Rename {
        title: request.name,
    };
    action.validate().map_err(map_action_error)?;
    dispatch_action(&state, &snapshot, Some(Target::Id(id)), action).await?;
    let snapshot = read_snapshot(&state)?;
    let session = find_session(&snapshot, id)?;
    Ok(Json(ActionResponse {
        ok: true,
        tab: tab_summary(session, snapshot.active_id),
        detail: format!("Renamed tab to '{}'.", session.title),
        delivery: None,
        bytes_sent: None,
    }))
}

async fn open_layout(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Json(request): Json<OpenLayoutRequest>,
) -> Result<Json<LayoutResponse>, (StatusCode, Json<serde_json::Value>)> {
    authorize(&headers, &state)?;
    if !(2..=32).contains(&request.names.len()) {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "open-layout requires between 2 and 32 tab names",
        ));
    }
    let mode = match request.layout.as_str() {
        "horizontal" => LayoutMode::Columns,
        "vertical" => LayoutMode::Rows,
        "grid" => LayoutMode::Grid,
        _ => {
            return Err(api_error(
                StatusCode::BAD_REQUEST,
                "layout must be one of: horizontal, vertical, grid",
            ));
        }
    };
    if request
        .columns
        .is_some_and(|columns| !(1..=10).contains(&columns))
    {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "columns must be between 1 and 10",
        ));
    }
    if request.columns.is_some() && request.layout != "grid" {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "columns can only be used with a grid layout",
        ));
    }
    let actions = request
        .names
        .iter()
        .map(|name| Action::CreateNamed {
            name: name.clone(),
            shell: request.shell.clone(),
            cwd: request.cwd.clone(),
        })
        .collect::<Vec<_>>();
    for action in &actions {
        action.validate().map_err(map_action_error)?;
    }

    let mut created_ids = Vec::with_capacity(actions.len());
    for action in actions {
        let snapshot = read_snapshot(&state)?;
        match dispatch_action(&state, &snapshot, None, action).await {
            Ok(Some(id)) => created_ids.push(id),
            Ok(None) => {
                rollback_created_tabs(&state, &created_ids).await;
                return Err(api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "created tab has no ID",
                ));
            }
            Err(error) => {
                rollback_created_tabs(&state, &created_ids).await;
                return Err(error);
            }
        }
    }

    let snapshot = read_snapshot(&state)?;
    if let Err(error) = dispatch_action(&state, &snapshot, None, Action::Layout { mode }).await {
        rollback_created_tabs(&state, &created_ids).await;
        return Err(error);
    }
    let snapshot = read_snapshot(&state)?;
    if let Some(columns) = request.columns {
        if let Err(error) = dispatch_action(
            &state,
            &snapshot,
            None,
            Action::GridColumns {
                columns: Some(columns),
            },
        )
        .await
        {
            rollback_created_tabs(&state, &created_ids).await;
            return Err(error);
        }
    }
    let snapshot = read_snapshot(&state)?;
    if let Err(error) = dispatch_action(
        &state,
        &snapshot,
        None,
        Action::ShowTabs {
            ids: created_ids.iter().take(10).copied().collect(),
        },
    )
    .await
    {
        rollback_created_tabs(&state, &created_ids).await;
        return Err(error);
    }
    let snapshot = read_snapshot(&state)?;
    let tabs = created_ids
        .iter()
        .filter_map(|id| find_session(&snapshot, *id).ok())
        .map(|session| tab_summary(session, snapshot.active_id))
        .collect::<Vec<_>>();
    let active_tab_id = snapshot.active_id.map(|id| format!("tab-{id}"));
    let visible_tab_ids = snapshot
        .visible_ids
        .iter()
        .map(|id| format!("tab-{id}"))
        .collect();
    Ok(Json(LayoutResponse {
        ok: true,
        detail: format!(
            "Opened {} tabs in a {} layout.",
            created_ids.len(),
            request.layout
        ),
        layout: request.layout,
        columns: request.columns,
        tabs,
        active_tab_id,
        visible_tab_ids,
    }))
}

async fn presets(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> Result<Json<PresetsResponse>, (StatusCode, Json<serde_json::Value>)> {
    authorize(&headers, &state)?;
    let snapshot = read_snapshot(&state)?;
    let mut presets = snapshot.presets;
    presets.sort_by(|left, right| {
        left.kind.cmp(&right.kind).then_with(|| {
            left.label
                .to_ascii_lowercase()
                .cmp(&right.label.to_ascii_lowercase())
        })
    });
    Ok(Json(PresetsResponse {
        presets: presets.into_iter().map(preset_summary).collect(),
    }))
}

async fn run_preset(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Json(request): Json<PresetRunRequest>,
) -> Result<Json<ActionResponse>, (StatusCode, Json<serde_json::Value>)> {
    authorize(&headers, &state)?;
    let snapshot = read_snapshot(&state)?;
    let needle = request.label.trim().to_ascii_lowercase();
    if needle.is_empty() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "preset label is required",
        ));
    }
    let mut matches = snapshot
        .presets
        .iter()
        .filter(|preset| preset.label.trim().to_ascii_lowercase() == needle);
    let preset = matches
        .next()
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "preset was not found"))?;
    if matches.next().is_some() {
        return Err(api_error(StatusCode::CONFLICT, "preset label is ambiguous"));
    }
    let mut bytes = preset.command.as_bytes().to_vec();
    if preset.send_enter {
        bytes.push(b'\r');
    }
    if bytes.is_empty() || bytes.len() > MAX_PAYLOAD_BYTES || bytes.contains(&0) {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "preset input is empty or exceeds 64 KiB",
        ));
    }
    let target = Target::Selector(request.tab.unwrap_or_else(|| "active".into()));
    let id = snapshot.resolve(&target).map_err(map_action_error)?;
    dispatch_action(&state, &snapshot, Some(Target::Id(id)), Action::Send(bytes)).await?;
    let snapshot = read_snapshot(&state)?;
    let session = find_session(&snapshot, id)?;
    Ok(Json(ActionResponse {
        ok: true,
        tab: tab_summary(session, snapshot.active_id),
        detail: format!("Ran {} '{}'.", preset.kind, preset.label),
        delivery: None,
        bytes_sent: None,
    }))
}

async fn read_tab(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    AxumPath(selector): AxumPath<String>,
    Query(query): Query<ReadQuery>,
) -> Result<Json<ReadResponse>, (StatusCode, Json<serde_json::Value>)> {
    authorize(&headers, &state)?;
    if query.chars.is_some() && query.lines.is_some() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "choose either chars or lines, not both",
        ));
    }
    let snapshot = read_snapshot(&state)?;
    let session = resolve_session(&snapshot, &selector)?;
    let from_top = query
        .from
        .as_deref()
        .is_some_and(|value| value.eq_ignore_ascii_case("top"));
    let text = if let Some(lines) = query.lines {
        session
            .output_capture
            .read_lines(lines.min(MAX_READ_CHARS), from_top)
    } else {
        session.output_capture.read_chars(
            query
                .chars
                .unwrap_or(DEFAULT_READ_CHARS)
                .min(MAX_READ_CHARS),
            from_top,
        )
    };
    Ok(Json(ReadResponse {
        tab: tab_summary(session, snapshot.active_id),
        text,
    }))
}

async fn send_tab(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    AxumPath(selector): AxumPath<String>,
    Json(request): Json<PayloadRequest>,
) -> Result<Json<ActionResponse>, (StatusCode, Json<serde_json::Value>)> {
    authorize(&headers, &state)?;
    let (bytes, _) = resolve_payload(request.text, request.payload_base64)?;
    let mode = resolve_delivery(request.delivery.as_deref())?;
    let prepared =
        crate::session::input::prepare_delivery(&bytes, request.enter, mode, request.delay_ms)
            .map_err(|message| api_error(StatusCode::BAD_REQUEST, message))?;
    let snapshot = read_snapshot(&state)?;
    let target_id = resolve_session(&snapshot, &selector)?.id;
    let bytes_sent = prepared.chunks.iter().map(Vec::len).sum();
    let delivery = prepared.mode.name();
    if bytes_sent == 0 || bytes_sent > MAX_PAYLOAD_BYTES {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "input must be between 1 byte and 64 KiB",
        ));
    }
    execute_delivery(&state, target_id, prepared).await?;
    let snapshot = read_snapshot(&state)?;
    let session = find_session(&snapshot, target_id)?;
    Ok(Json(ActionResponse {
        ok: true,
        tab: tab_summary(session, snapshot.active_id),
        detail: "Sent payload to terminal.".into(),
        delivery: Some(delivery.into()),
        bytes_sent: Some(bytes_sent),
    }))
}

async fn send_key(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    AxumPath(selector): AxumPath<String>,
    Json(request): Json<KeyRequest>,
) -> Result<Json<ActionResponse>, (StatusCode, Json<serde_json::Value>)> {
    authorize(&headers, &state)?;
    let bytes = key_sequence(&request.key).ok_or_else(|| {
        api_error(
            StatusCode::BAD_REQUEST,
            "unsupported key; supported keys are ctrl+c, ctrl+d, ctrl+z, enter, tab, and escape",
        )
    })?;
    let snapshot = read_snapshot(&state)?;
    let target_id = resolve_session(&snapshot, &selector)?.id;
    execute_send(&state, target_id, bytes.to_vec(), Duration::from_secs(30)).await?;
    let snapshot = read_snapshot(&state)?;
    let session = find_session(&snapshot, target_id)?;
    Ok(Json(ActionResponse {
        ok: true,
        tab: tab_summary(session, snapshot.active_id),
        detail: format!("Sent key '{}'.", request.key),
        delivery: None,
        bytes_sent: None,
    }))
}

async fn run_command(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    AxumPath(selector): AxumPath<String>,
    Json(request): Json<RunRequest>,
) -> Result<Json<RunResponse>, (StatusCode, Json<serde_json::Value>)> {
    authorize(&headers, &state)?;
    let (mut bytes, _) = resolve_payload(request.text.clone(), request.payload_base64)?;
    let mode = resolve_delivery(request.delivery.as_deref())?;
    if let Some(text) = request.text {
        let trimmed = text.trim_end_matches(['\r', '\n']).to_owned();
        if trimmed.trim().is_empty() {
            return Err(api_error(
                StatusCode::BAD_REQUEST,
                "run payload is required",
            ));
        }
        bytes = trimmed.into_bytes();
    }
    let prepared =
        crate::session::input::prepare_delivery(&bytes, request.enter, mode, request.delay_ms)
            .map_err(|message| api_error(StatusCode::BAD_REQUEST, message))?;
    let bytes_sent = prepared.chunks.iter().map(Vec::len).sum();
    let delivery = prepared.mode.name();
    if bytes_sent == 0 || bytes_sent > MAX_PAYLOAD_BYTES {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "run input must be between 1 byte and 64 KiB",
        ));
    }
    let quiet_ms = request.quiet_ms.unwrap_or(DEFAULT_RUN_QUIET_MS);
    let max_wait_ms = request
        .max_wait_ms
        .unwrap_or(DEFAULT_RUN_MAX_WAIT_MS)
        .min(60_000);
    let interval_ms = request
        .interval_ms
        .unwrap_or(DEFAULT_RUN_INTERVAL_MS)
        .clamp(25, 2_000);
    if quiet_ms == 0 || max_wait_ms == 0 || request.interval_ms == Some(0) {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "quietMs, maxWaitMs, and intervalMs must be greater than 0",
        ));
    }
    let chars = request
        .chars
        .unwrap_or(DEFAULT_RUN_CHARS)
        .clamp(1, MAX_READ_CHARS);
    let snapshot = read_snapshot(&state)?;
    let target_id = resolve_session(&snapshot, &selector)?.id;
    execute_delivery(&state, target_id, prepared).await?;
    let started = std::time::Instant::now();
    let deadline = started + Duration::from_millis(max_wait_ms);
    let mut previous_signature = String::new();
    let mut stable_since = started;
    let completion_reason;
    let mut matched_text = None;
    let mut timed_out = false;
    let text;
    loop {
        let snapshot = read_snapshot(&state)?;
        let session = find_session(&snapshot, target_id)?;
        let output = session.output_capture.snapshot();
        let current_text = session.output_capture.read_chars(chars, false);
        let signature = format!("{}:{current_text}", output.output_sequence);
        let now = std::time::Instant::now();
        if signature != previous_signature {
            previous_signature = signature;
            stable_since = now;
        }
        if let Some(needle) = request.wait_for_text.as_ref() {
            let matches = if request.ignore_case {
                current_text
                    .to_ascii_lowercase()
                    .contains(&needle.to_ascii_lowercase())
            } else {
                current_text.contains(needle)
            };
            if matches {
                matched_text = Some(needle.clone());
                completion_reason = "matched-text".into();
                text = current_text;
                break;
            }
        }
        if now.duration_since(stable_since) >= Duration::from_millis(quiet_ms) {
            completion_reason = "quiet".into();
            text = current_text;
            break;
        }
        if now >= deadline {
            timed_out = true;
            completion_reason = "timeout".into();
            text = current_text;
            break;
        }
        tokio::time::sleep(Duration::from_millis(interval_ms)).await;
    }
    let elapsed_ms = started.elapsed().as_millis().min(u64::MAX as u128) as u64;
    let snapshot = read_snapshot(&state)?;
    let session = find_session(&snapshot, target_id)?;
    Ok(Json(RunResponse {
        ok: true,
        tab: tab_summary(session, snapshot.active_id),
        detail: format!("Ran payload on tab '{selector}'."),
        delivery: delivery.into(),
        bytes_sent,
        text,
        chars,
        elapsed_ms,
        timed_out,
        stable: !timed_out,
        completion_reason,
        matched_text,
    }))
}

fn authorize(
    headers: &HeaderMap,
    state: &ApiState,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    if !crate::app::remote_control_available() {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            "remote control is not available for this installation",
        ));
    }
    if headers.contains_key(header::ORIGIN) {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            "browser-origin requests are not allowed",
        ));
    }
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok());
    if host != state.base_url.strip_prefix("http://") {
        return Err(api_error(StatusCode::FORBIDDEN, "invalid control API host"));
    }
    let token = headers
        .get(header::AUTHORIZATION)
        .map(|value| value.as_bytes())
        .unwrap_or_default();
    let expected = format!("Bearer {}", state.token.as_str());
    if constant_time_eq(token, expected.as_bytes()) {
        Ok(())
    } else {
        Err(api_error(
            StatusCode::UNAUTHORIZED,
            "missing or invalid control API token",
        ))
    }
}

fn read_snapshot(state: &ApiState) -> Result<Snapshot, (StatusCode, Json<serde_json::Value>)> {
    state
        .snapshot
        .read()
        .map(|snapshot| snapshot.clone())
        .map_err(|_| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "session state unavailable",
            )
        })
}

fn resolve_session<'a>(
    snapshot: &'a Snapshot,
    selector: &str,
) -> Result<&'a crate::session::actions::SessionInfo, (StatusCode, Json<serde_json::Value>)> {
    let id = snapshot
        .resolve(&Target::Selector(selector.to_owned()))
        .map_err(map_action_error)?;
    find_session(snapshot, id)
}

fn find_session(
    snapshot: &Snapshot,
    id: u64,
) -> Result<&crate::session::actions::SessionInfo, (StatusCode, Json<serde_json::Value>)> {
    snapshot
        .sessions
        .iter()
        .find(|session| session.id == id)
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "terminal was not found"))
}

fn tab_summary(
    session: &crate::session::actions::SessionInfo,
    active_id: Option<u64>,
) -> TabSummary {
    let output = session.output_capture.snapshot();
    TabSummary {
        tab_id: format!("tab-{}", session.id),
        pty_id: session.id,
        title: session.title.clone(),
        shell: session.shell.clone(),
        is_active: active_id == Some(session.id),
        stored_output_chars: output.stored_output_chars,
        output_truncated: output.output_truncated,
        last_input: output.last_input,
        last_updated_at_ms: output.last_updated_at_ms,
        last_input_at_ms: output.last_input_at_ms,
        last_output_at_ms: output.last_output_at_ms,
        output_sequence: output.output_sequence,
        ready: session.ready,
        exited: session.exited,
    }
}

fn preset_summary(preset: PresetInfo) -> PresetSummary {
    PresetSummary {
        kind: preset.kind,
        label: preset.label,
        command: preset.command,
        send_enter: preset.send_enter,
    }
}

fn resolve_payload(
    text: Option<String>,
    payload_base64: Option<String>,
) -> Result<(Vec<u8>, &'static str), (StatusCode, Json<serde_json::Value>)> {
    match (text, payload_base64) {
        (Some(text), None) => {
            let bytes = text.into_bytes();
            validate_payload(&bytes)?;
            Ok((bytes, "text"))
        }
        (None, Some(encoded)) => {
            let bytes = BASE64.decode(encoded.as_bytes()).map_err(|_| {
                api_error(
                    StatusCode::BAD_REQUEST,
                    "payloadBase64 must be valid base64",
                )
            })?;
            validate_payload(&bytes)?;
            Ok((bytes, "base64"))
        }
        (Some(_), Some(_)) => Err(api_error(
            StatusCode::BAD_REQUEST,
            "provide exactly one payload source: text or payloadBase64",
        )),
        (None, None) => Err(api_error(
            StatusCode::BAD_REQUEST,
            "provide exactly one payload source: text or payloadBase64",
        )),
    }
}

fn validate_payload(bytes: &[u8]) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    if bytes.len() > MAX_PAYLOAD_BYTES || bytes.contains(&0) {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "input must be at most 64 KiB and must not contain NUL",
        ));
    }
    Ok(())
}

fn resolve_delivery(
    requested: Option<&str>,
) -> Result<crate::session::input::DeliveryMode, (StatusCode, Json<serde_json::Value>)> {
    match requested.unwrap_or("raw") {
        "raw" => Ok(crate::session::input::DeliveryMode::Raw),
        "bracketed" => Ok(crate::session::input::DeliveryMode::Bracketed),
        "slow-typed" => Ok(crate::session::input::DeliveryMode::SlowTyped),
        _ => Err(api_error(
            StatusCode::BAD_REQUEST,
            "delivery must be one of: raw, bracketed, slow-typed",
        )),
    }
}

async fn execute_send(
    state: &ApiState,
    target_id: u64,
    bytes: Vec<u8>,
    timeout: Duration,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    let snapshot = read_snapshot(state)?;
    let pending = state
        .dispatcher
        .submit(
            &snapshot,
            Some(Target::Id(target_id)),
            Action::Send(bytes),
            crate::app::remote_control_available(),
            timeout,
        )
        .map_err(map_action_error)?;
    state.repaint.request_repaint();
    tokio::task::spawn_blocking(move || pending.recv_timeout(timeout))
        .await
        .map_err(|_| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "terminal action worker failed",
            )
        })?
        .map(|_| ())
        .map_err(map_action_error)
}

async fn execute_delivery(
    state: &ApiState,
    target_id: u64,
    prepared: crate::session::input::PreparedDelivery,
) -> Result<usize, (StatusCode, Json<serde_json::Value>)> {
    let total_bytes = prepared.chunks.iter().map(Vec::len).sum();
    let slow_deadline = (prepared.mode == crate::session::input::DeliveryMode::SlowTyped)
        .then(|| tokio::time::Instant::now() + Duration::from_secs(30));
    for (index, chunk) in prepared.chunks.iter().enumerate() {
        let timeout = slow_deadline
            .map(|deadline| deadline.saturating_duration_since(tokio::time::Instant::now()))
            .unwrap_or(Duration::from_secs(30));
        if timeout.is_zero() {
            return Err(map_action_error(ActionError::Timeout));
        }
        execute_send(state, target_id, chunk.clone(), timeout).await?;
        if prepared.mode == crate::session::input::DeliveryMode::SlowTyped
            && index + 1 < prepared.chunks.len()
        {
            let delay = Duration::from_millis(prepared.delay_ms);
            if let Some(deadline) = slow_deadline {
                let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
                if delay >= remaining {
                    tokio::time::sleep(remaining).await;
                    return Err(map_action_error(ActionError::Timeout));
                }
            }
            tokio::time::sleep(delay).await;
        }
    }
    let snapshot = read_snapshot(state)?;
    find_session(&snapshot, target_id)?
        .output_capture
        .record_input_bytes(&prepared.logical_input);
    Ok(total_bytes)
}

async fn dispatch_action(
    state: &ApiState,
    snapshot: &Snapshot,
    target: Option<Target>,
    action: Action,
) -> Result<Option<u64>, (StatusCode, Json<serde_json::Value>)> {
    let pending = state
        .dispatcher
        .submit(
            snapshot,
            target,
            action,
            crate::app::remote_control_available(),
            Duration::from_secs(30),
        )
        .map_err(map_action_error)?;
    state.repaint.request_repaint();
    tokio::task::spawn_blocking(move || pending.recv_timeout(Duration::from_secs(30)))
        .await
        .map_err(|_| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "terminal action worker failed",
            )
        })?
        .map_err(map_action_error)
}

async fn rollback_created_tabs(state: &ApiState, created_ids: &[u64]) {
    for id in created_ids.iter().rev() {
        if let Ok(snapshot) = read_snapshot(state) {
            let _ = dispatch_action(state, &snapshot, Some(Target::Id(*id)), Action::Close).await;
        }
    }
}

fn key_sequence(key: &str) -> Option<&'static [u8]> {
    match key.trim().to_ascii_lowercase().as_str() {
        "ctrl+c" | "^c" => Some(b"\x03"),
        "ctrl+d" | "^d" => Some(b"\x04"),
        "ctrl+z" | "^z" => Some(b"\x1a"),
        "enter" | "return" => Some(b"\r"),
        "tab" => Some(b"\t"),
        "escape" | "esc" => Some(b"\x1b"),
        _ => None,
    }
}

fn map_action_error(error: ActionError) -> (StatusCode, Json<serde_json::Value>) {
    let status = match error {
        ActionError::DeniedAccess => StatusCode::FORBIDDEN,
        ActionError::NotFound | ActionError::NoActive => StatusCode::NOT_FOUND,
        ActionError::AmbiguousTitle => StatusCode::CONFLICT,
        ActionError::NotReady | ActionError::Exited | ActionError::Closed => StatusCode::CONFLICT,
        ActionError::Timeout => StatusCode::GATEWAY_TIMEOUT,
        ActionError::QueueFull | ActionError::ShuttingDown => StatusCode::SERVICE_UNAVAILABLE,
        ActionError::InvalidInput => StatusCode::BAD_REQUEST,
        ActionError::Unsupported => StatusCode::NOT_IMPLEMENTED,
        ActionError::LaunchFailed => StatusCode::INTERNAL_SERVER_ERROR,
    };
    api_error(status, &error.to_string())
}

fn default_true() -> bool {
    true
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

fn api_error(status: StatusCode, message: &str) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(json!({ "error": message })))
}

fn write_discovery_file(path: &Path, descriptor: &DiscoveryFile) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(descriptor).map_err(|error| error.to_string())?;
    if bytes.len() > MAX_BODY_BYTES {
        return Err("control discovery descriptor exceeded its limit".into());
    }
    let temp_path = path.with_extension("json.tmp");
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&temp_path)
        .map_err(|error| error.to_string())?;
    file.write_all(&bytes).map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())?;
    fs::rename(&temp_path, path).map_err(|error| {
        let _ = fs::remove_file(&temp_path);
        error.to_string()
    })
}

fn set_private_directory_permissions(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|error| error.to_string())?;
    }
    #[cfg(windows)]
    {
        let _ = path;
        // The native root is under the current user's home and inherits its ACL.
    }
    Ok(())
}

fn random_hex(byte_count: usize) -> Result<String, String> {
    let mut bytes = vec![0; byte_count];
    getrandom::fill(&mut bytes).map_err(|error| format!("system random source failed: {error}"))?;
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(byte_count.saturating_mul(2));
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    Ok(output)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or_default()
}
