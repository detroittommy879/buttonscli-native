//! Optional loopback-only web preview. Provider secrets stay in OS credentials.
use crate::assistant::{
    client::ClientError,
    credentials::{self, CredentialStore, SystemCredentialStore},
    provider::{AiTask, ProviderProfile},
    simulation::generate_turn,
};
use crate::settings::SimulationPreferences;
use axum::{
    extract::{DefaultBodyLimit, Path, Request, State},
    http::{header, HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::path::{Component, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

#[derive(Clone)]
struct ServerState {
    root: PathBuf,
    origin: String,
    host: String,
    token: String,
    profile: String,
    config: SimulationPreferences,
    providers: Vec<ProviderProfile>,
    slots: Arc<tokio::sync::Semaphore>,
    requests: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TurnRequest {
    client_id: String,
    id: u64,
    system: String,
    user: String,
}

struct CancelGuard {
    cancel: Arc<AtomicBool>,
    key: String,
    requests: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
}
impl Drop for CancelGuard {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        self.requests.lock().unwrap().remove(&self.key);
    }
}

async fn guard(State(state): State<ServerState>, request: Request, next: Next) -> Response {
    if !allowed_origin(&state, request.headers()) {
        return (StatusCode::FORBIDDEN, "Use this preview's loopback URL.").into_response();
    }
    let path = request.uri().path();
    if path.starts_with("/api/")
        && path != "/api/session"
        && request
            .headers()
            .get("x-buttonscli-preview")
            .and_then(|value| value.to_str().ok())
            != Some(state.token.as_str())
    {
        return (
            StatusCode::UNAUTHORIZED,
            "Preview session token is missing.",
        )
            .into_response();
    }
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    headers.insert("x-content-type-options", "nosniff".parse().unwrap());
    headers.insert("x-frame-options", "DENY".parse().unwrap());
    headers.insert("content-security-policy", "default-src 'self'; script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'".parse().unwrap());
    response
}
fn allowed_origin(state: &ServerState, headers: &HeaderMap) -> bool {
    headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        == Some(state.host.as_str())
        && headers
            .get(header::ORIGIN)
            .is_none_or(|value| value.to_str().ok() == Some(state.origin.as_str()))
        && headers
            .get("sec-fetch-site")
            .is_none_or(|value| matches!(value.to_str().ok(), Some("same-origin" | "none")))
}
async fn session(State(state): State<ServerState>) -> Json<serde_json::Value> {
    let model = state
        .providers
        .first()
        .map(|p| format!("{} · {}", p.name, p.model));
    Json(
        json!({"token":state.token,"model":model,"available":state.providers.first().is_some_and(|p| !p.endpoint.is_empty() && !p.model.is_empty()),"config":state.config}),
    )
}
async fn turn(State(state): State<ServerState>, Json(request): Json<TurnRequest>) -> Response {
    if request.client_id.len() > 64
        || request.client_id.is_empty()
        || !request
            .client_id
            .bytes()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == b'-')
        || request.id == 0
        || request.system.trim().is_empty()
        || request.user.trim().is_empty()
        || request.system.len() > 32 * 1024
        || request.user.len() > 256 * 1024
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"Invalid or oversized simulator request."})),
        )
            .into_response();
    }
    let Ok(slot) = state.slots.clone().try_acquire_owned() else {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({"error":"Preview is busy; try again after the current request."})),
        )
            .into_response();
    };
    let key = format!("{}:{}", request.client_id, request.id);
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut requests = state.requests.lock().unwrap();
        if requests.contains_key(&key) {
            return (
                StatusCode::CONFLICT,
                Json(json!({"error":"Request already running."})),
            )
                .into_response();
        }
        requests.insert(key.clone(), cancel.clone());
    }
    let guard = CancelGuard {
        cancel: cancel.clone(),
        key,
        requests: state.requests.clone(),
    };
    let worker = tokio::task::spawn_blocking(move || {
        let _slot = slot;
        generate_turn(
            &crate::assistant::transport::ReqwestTransport,
            &state.providers,
            &mut |provider| {
                let reference = credentials::reference(&state.profile, &provider.id);
                if provider.credential_ref.as_deref() != Some(reference.as_str()) {
                    return Ok(None);
                }
                SystemCredentialStore
                    .get(&reference)
                    .map(Some)
                    .map_err(|_| ClientError::Credential)
            },
            (&request.system, &request.user),
            &cancel,
        )
    });
    let result = tokio::time::timeout(Duration::from_secs(185), worker).await;
    drop(guard);
    match result {
        Ok(Ok(Ok(turn))) => {
            Json(json!({"turn":{"output":turn.output,"state":turn.state},"model":turn.model}))
                .into_response()
        }
        Ok(Ok(Err(error))) => {
            (StatusCode::BAD_GATEWAY, Json(json!({"error":error}))).into_response()
        }
        _ => (
            StatusCode::GATEWAY_TIMEOUT,
            Json(
                json!({"error":"Simulator request stopped or timed out. Virtual state was kept."}),
            ),
        )
            .into_response(),
    }
}
async fn cancel(
    State(state): State<ServerState>,
    Path((client, id)): Path<(String, u64)>,
) -> StatusCode {
    if let Some(cancel) = state
        .requests
        .lock()
        .unwrap()
        .get(&format!("{client}:{id}"))
    {
        cancel.store(true, Ordering::Relaxed);
    }
    StatusCode::NO_CONTENT
}
async fn index(State(state): State<ServerState>) -> Response {
    asset(state, "index.html".into()).await
}
async fn file(State(state): State<ServerState>, Path(path): Path<String>) -> Response {
    asset(state, path).await
}
async fn asset(state: ServerState, path: String) -> Response {
    if !matches!(
        path.as_str(),
        "index.html"
            | "simulator.js"
            | "build-info.json"
            | "pkg/buttonscli.js"
            | "pkg/buttonscli_bg.wasm"
    ) || !std::path::Path::new(&path)
        .components()
        .all(|part| matches!(part, Component::Normal(_)))
    {
        return StatusCode::NOT_FOUND.into_response();
    }
    let target = state.root.join(&path);
    let Ok(target) = target.canonicalize() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if !target.starts_with(&state.root) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Ok(bytes) = std::fs::read(&target) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let content_type = match target.extension().and_then(|p| p.to_str()) {
        Some("wasm") => "application/wasm",
        Some("js") => "text/javascript; charset=utf-8",
        Some("json") => "application/json",
        _ => "text/html; charset=utf-8",
    };
    ([(header::CONTENT_TYPE, content_type)], bytes).into_response()
}
fn router(state: ServerState) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/api/session", get(session))
        .route("/api/simulation", post(turn))
        .route("/api/simulation/{client}/{id}", delete(cancel))
        .route("/{*path}", get(file))
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(middleware::from_fn_with_state(state.clone(), guard))
        .with_state(state)
}

/// Serve assets and an explicitly launched simulator bridge on 127.0.0.1 only.
pub fn serve(root: PathBuf, port: u16) -> Result<(), String> {
    let root = root
        .canonicalize()
        .map_err(|_| "Web folder not found. Build the WASM preview first.")?;
    if !root.join("pkg/buttonscli_bg.wasm").is_file() {
        return Err("Build the WASM preview first: scripts/build-web.ps1".into());
    }
    let (native, legacy) =
        crate::storage::paths::production_roots().map_err(|error| error.to_string())?;
    let store = crate::storage::store::NativeStore::open(native, legacy.0)
        .map_err(|error| error.to_string())?;
    let preferences = store
        .load()
        .map_err(|error| error.to_string())?
        .map(|loaded| loaded.preferences)
        .unwrap_or_default();
    let mut random = [0u8; 32];
    getrandom::fill(&mut random).map_err(|_| "Could not create preview session token.")?;
    let token = random
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let state = ServerState {
        root,
        origin: format!("http://127.0.0.1:{port}"),
        host: format!("127.0.0.1:{port}"),
        token,
        profile: store.profile_name().into(),
        config: preferences.simulation,
        providers: preferences.provider_settings.chain(AiTask::Simulation),
        slots: Arc::new(tokio::sync::Semaphore::new(2)),
        requests: Arc::new(Mutex::new(HashMap::new())),
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| error.to_string())?;
    runtime.block_on(async move {
        let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))
            .await
            .map_err(|error| error.to_string())?;
        println!("ButtonsCLI preview: http://127.0.0.1:{port}");
        axum::serve(listener, router(state))
            .await
            .map_err(|error| error.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authenticated_bridge_roundtrip_preserves_host_files_and_blocks_other_routes() {
        use std::io::BufRead;
        struct Provider(std::process::Child);
        impl Drop for Provider {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        let mut provider = Provider(
            std::process::Command::new("node")
                .arg("scripts/fake-ai-provider.mjs")
                .stdout(std::process::Stdio::piped())
                .spawn()
                .unwrap(),
        );
        let mut ready = String::new();
        std::io::BufReader::new(provider.0.stdout.take().unwrap())
            .read_line(&mut ready)
            .unwrap();
        let ready: serde_json::Value = serde_json::from_str(&ready).unwrap();
        let owned =
            std::env::temp_dir().join(format!("buttonscli-web-bridge-{:032x}", fastrand::u128(..)));
        std::fs::create_dir_all(&owned).unwrap();
        std::fs::write(owned.join("index.html"), "owned web fixture").unwrap();
        std::fs::write(owned.join("host-sentinel.txt"), "must survive").unwrap();
        let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
        let addr = listener.local_addr().unwrap();
        listener.set_nonblocking(true).unwrap();
        let base = format!("http://{addr}");
        let state = ServerState {
            root: owned.canonicalize().unwrap(),
            origin: base.clone(),
            host: addr.to_string(),
            token: "fixture-token".into(),
            profile: "owned-fixture".into(),
            config: Default::default(),
            providers: vec![ProviderProfile {
                endpoint: ready["endpoint"].as_str().unwrap().into(),
                model: "fake-simulation".into(),
                ..Default::default()
            }],
            slots: Arc::new(tokio::sync::Semaphore::new(2)),
            requests: Default::default(),
        };
        let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
        let server = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async move {
                let listener = tokio::net::TcpListener::from_std(listener).unwrap();
                axum::serve(listener, router(state))
                    .with_graceful_shutdown(async {
                        let _ = stopped.await;
                    })
                    .await
                    .unwrap();
            });
        });
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();
        let session: serde_json::Value = client
            .get(format!("{base}/api/session"))
            .send()
            .unwrap()
            .json()
            .unwrap();
        assert_eq!(session["model"], "OpenRouter · fake-simulation");
        let (system, user) = crate::assistant::simulation_protocol::build_prompts(
            &Default::default(),
            "rm -rf /",
            &Default::default(),
            &[],
            false,
        )
        .unwrap();
        let body = json!({"client_id":"owned-client","id":1,"system":system,"user":user});
        assert_eq!(
            client
                .post(format!("{base}/api/simulation"))
                .json(&body)
                .send()
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            client
                .post(format!("{base}/api/simulation"))
                .header("x-buttonscli-preview", "fixture-token")
                .header(header::ORIGIN, "https://outside.example")
                .json(&body)
                .send()
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );
        let reply: serde_json::Value = client
            .post(format!("{base}/api/simulation"))
            .header("x-buttonscli-preview", "fixture-token")
            .json(&body)
            .send()
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .unwrap();
        assert!(reply["turn"]["state"]["files"]
            .as_object()
            .unwrap()
            .is_empty());
        assert_eq!(
            std::fs::read_to_string(owned.join("host-sentinel.txt")).unwrap(),
            "must survive"
        );
        assert_eq!(
            client
                .get(format!("{base}/host-sentinel.txt"))
                .send()
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            client
                .get(format!("{base}/api/status"))
                .header("x-buttonscli-preview", "fixture-token")
                .send()
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            client
                .get(format!("{base}/"))
                .header(header::HOST, "rebound.example")
                .send()
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );
        stop.send(()).unwrap();
        server.join().unwrap();
        assert!(owned
            .canonicalize()
            .unwrap()
            .starts_with(std::env::temp_dir().canonicalize().unwrap()));
        std::fs::remove_dir_all(owned).unwrap();
    }

    #[test]
    fn bridge_rejects_cross_origin_and_rebound_hosts() {
        let state = ServerState {
            root: PathBuf::new(),
            origin: "http://127.0.0.1:8080".into(),
            host: "127.0.0.1:8080".into(),
            token: "fixture".into(),
            profile: "owned".into(),
            config: Default::default(),
            providers: vec![],
            slots: Arc::new(tokio::sync::Semaphore::new(2)),
            requests: Default::default(),
        };
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, "127.0.0.1:8080".parse().unwrap());
        assert!(allowed_origin(&state, &headers));
        headers.insert(header::ORIGIN, "https://attacker.example".parse().unwrap());
        assert!(!allowed_origin(&state, &headers));
        headers.remove(header::ORIGIN);
        headers.insert("sec-fetch-site", "cross-site".parse().unwrap());
        assert!(!allowed_origin(&state, &headers));
        headers.remove("sec-fetch-site");
        headers.insert(header::HOST, "rebound.example:8080".parse().unwrap());
        assert!(!allowed_origin(&state, &headers));
    }
}
