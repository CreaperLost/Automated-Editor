//! The local MCP server: AI agents the user runs themselves (the Claude app, Claude Code,
//! Codex, Gemini CLI, Cursor, ...) connect to AeroEdits here and work on the open project.
//! AeroEdits calls no model of its own on this path; the agent's own plan pays for it.
//!
//! An agent starts `aeroedits --mcp-stdio` ([`bridge`]), which reaches this app over a
//! private pipe ([`ipc`]): only this computer's account that runs the app can open it, and
//! no network port is involved, so there is no token to keep. No tool runs scripts or
//! touches files outside the project. It is off until the user turns it on.
mod actions;
pub mod bridge;
pub mod cli;
pub mod clients;
pub(crate) mod frames;
pub mod ipc;
mod layouts;
mod propose;
pub mod skill;
pub mod storyboard;
#[cfg(test)]
pub(crate) mod tests_support;
pub mod tools;
mod views;

use crate::commands::AppState;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

const SETTINGS_FILE: &str = "mcp.json";
const ACTIVITY_KEPT: usize = 50;

/// What the server needs from the app around it.
pub trait Host: Send + Sync + 'static {
    fn state(&self) -> &AppState;
    /// Transcription and AI passes: one at a time, shared with the windows.
    fn transcripts(&self) -> &crate::commands::transcript::TranscriptState;
    /// The agent opened or created a project: the editor window shows it.
    fn project_opened(&self, _path: &str) {}
    /// A transcript changed (transcribed again): the windows reload words and captions.
    fn transcript_changed(&self) {}
    /// What agents are doing changed (a call started, progressed or ended).
    fn activity_changed(&self) {}
    /// The agent changed the project: the windows show the new revision.
    fn project_changed(&self) {}
    /// The agent proposed, withdrew or undid something: the Agent panel reloads.
    fn proposals_changed(&self) {}
    /// The agent started an export (of a short when `short`): the editor shows its progress as
    /// if started there.
    fn export_started(&self, _status: &crate::export::ExportStatus, _short: bool) {}
    /// Accept proposals as they arrive.
    fn autopilot(&self) -> bool {
        false
    }
    /// The app's AI review can run (a key is set): fillers use it unless told not to.
    fn ai_review_ready(&self) -> bool {
        actions::ai_key_ready()
    }
    /// The built-in local model is downloaded: fillers use it when no AI key is set.
    fn local_review_ready(&self) -> bool {
        crate::ai::local::ready()
    }
    /// The app's settings folder; the user's answers to templates are kept in it.
    fn config_dir(&self) -> std::path::PathBuf {
        crate::transcript::settings::config_dir()
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct McpSettings {
    pub enabled: bool,
    /// Agents' proposals are accepted as they arrive, without the user's review.
    pub autopilot: bool,
}

pub fn load_settings(dir: &Path) -> McpSettings {
    fs::read(dir.join(SETTINGS_FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<McpSettings>(&bytes).ok())
        .unwrap_or_default()
}

fn save_settings(dir: &Path, settings: &McpSettings) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(dir.join(SETTINGS_FILE), json).map_err(|e| e.to_string())
}

/// One tool call, for the activity list in the app.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub at: String,
    /// The agent's name for itself, such as `claude-ai` or `codex`.
    pub client: Option<String>,
    pub tool: String,
    pub ok: bool,
    pub millis: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct McpStatus {
    pub enabled: bool,
    pub autopilot: bool,
    pub running: bool,
    /// The pipe (or socket) agents reach through the bridge.
    pub endpoint: Option<String>,
    pub error: Option<String>,
    /// Agents connected now, by the name each gave.
    pub connected: Vec<String>,
    /// Newest first.
    pub activity: Vec<Activity>,
    /// Calls still running, oldest first.
    pub working: Vec<Working>,
    /// This program; agents start it as `<it> --mcp-stdio`.
    pub bridge_command: Option<String>,
    /// The agent apps AeroEdits can add itself to.
    pub apps: Vec<clients::AppState>,
    /// The `aeroedits` skill in agents that read skills from disk.
    pub skills: Vec<skill::SkillState>,
}

/// A call an agent is waiting on, for the Agent panel.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Working {
    #[serde(skip)]
    id: u64,
    pub client: Option<String>,
    pub tool: String,
    /// 0 to 1, once the call has said.
    pub fraction: Option<f64>,
    pub message: Option<String>,
}

struct Running {
    endpoint: ipc::Endpoint,
    stop: CancellationToken,
}

struct Server {
    running: Option<Running>,
    error: Option<String>,
    /// One entry per open connection: the agent's name once it has said it.
    connected: Vec<(u64, Option<String>)>,
    next_connection: u64,
    activity: VecDeque<Activity>,
    working: Vec<Working>,
    next_call: u64,
}

static SERVER: Mutex<Server> = Mutex::new(Server {
    running: None,
    error: None,
    connected: Vec::new(),
    next_connection: 0,
    activity: VecDeque::new(),
    working: Vec::new(),
    next_call: 0,
});

fn record(activity: Activity) {
    let mut server = SERVER.lock();
    server.activity.push_front(activity);
    server.activity.truncate(ACTIVITY_KEPT);
}

fn this_program() -> Option<std::path::PathBuf> {
    std::env::current_exe().ok()
}

pub fn status(dir: &Path) -> McpStatus {
    let settings = load_settings(dir);
    let exe = this_program();
    let apps = exe.as_deref().map(clients::states).unwrap_or_default();
    let server = SERVER.lock();
    McpStatus {
        enabled: settings.enabled,
        autopilot: settings.autopilot,
        running: server.running.is_some(),
        endpoint: server.running.as_ref().map(|r| r.endpoint.to_string()),
        error: server.error.clone(),
        connected: server
            .connected
            .iter()
            .map(|(_, name)| name.clone().unwrap_or_else(|| "agent".into()))
            .collect(),
        activity: server.activity.iter().cloned().collect(),
        working: server.working.clone(),
        bridge_command: exe.map(|p| p.to_string_lossy().into_owned()),
        apps,
        skills: skill::states(),
    }
}

/// Turns the server on or off, and remembers it for the next launch.
pub fn set_enabled(dir: &Path, host: Arc<dyn Host>, enabled: bool) -> Result<McpStatus, String> {
    save_settings(
        dir,
        &McpSettings {
            enabled,
            ..load_settings(dir)
        },
    )?;
    if enabled {
        start(host, ipc::default_endpoint()?)?;
    } else {
        stop();
    }
    Ok(status(dir))
}

/// Turns autopilot on or off: agents' proposals are accepted as they arrive.
pub fn set_autopilot(dir: &Path, autopilot: bool) -> Result<McpStatus, String> {
    save_settings(
        dir,
        &McpSettings {
            autopilot,
            ..load_settings(dir)
        },
    )?;
    Ok(status(dir))
}

/// Adds AeroEdits to an agent app's settings, or takes it out.
pub fn set_app_connected(
    dir: &Path,
    app: clients::App,
    connected: bool,
) -> Result<McpStatus, String> {
    let exe = this_program().ok_or("Could not find this program's path")?;
    if connected {
        clients::connect(app, &exe)?;
    } else {
        clients::disconnect(app, &exe)?;
    }
    Ok(status(dir))
}

/// Installs the `aeroedits` skill for an agent that reads skills from disk, or takes it out.
pub fn set_skill_installed(
    dir: &Path,
    home: skill::SkillHome,
    installed: bool,
) -> Result<McpStatus, String> {
    skill::set_installed(home, installed)?;
    Ok(status(dir))
}

/// Starts the server if the settings say so, and brings installed skills up to date; for app
/// launch.
pub fn start_if_enabled(dir: &Path, host: Arc<dyn Host>) {
    skill::refresh_installed();
    if load_settings(dir).enabled {
        if let Err(error) = ipc::default_endpoint().and_then(|endpoint| start(host, endpoint)) {
            eprintln!("[mcp] {error}");
        }
    }
}

pub fn stop() {
    if let Some(running) = SERVER.lock().running.take() {
        running.stop.cancel();
    }
}

fn start(host: Arc<dyn Host>, endpoint: ipc::Endpoint) -> Result<(), String> {
    stop();
    let result = serve(host, endpoint);
    let mut server = SERVER.lock();
    match result {
        Ok(running) => {
            server.running = Some(running);
            server.error = None;
            Ok(())
        }
        Err(error) => {
            server.error = Some(error.clone());
            Err(error)
        }
    }
}

/// Listens on `endpoint` on a thread of its own until stopped.
fn serve(host: Arc<dyn Host>, endpoint: ipc::Endpoint) -> Result<Running, String> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|e| format!("Could not start the agent server: {e}"))?;
    // Bound here, so a taken pipe is reported to the user rather than on a thread.
    let listener = {
        let _entered = runtime.enter();
        ipc::Listener::bind(&endpoint)?
    };
    let stop = CancellationToken::new();
    let cancelled = stop.clone();
    std::thread::Builder::new()
        .name("aeroedits-mcp".into())
        .spawn(move || runtime.block_on(accept(listener, host, cancelled)))
        .map_err(|e| format!("Could not start the agent server thread: {e}"))?;
    Ok(Running { endpoint, stop })
}

async fn accept(mut listener: ipc::Listener, host: Arc<dyn Host>, stop: CancellationToken) {
    loop {
        let stream = tokio::select! {
            stream = listener.accept() => stream,
            _ = stop.cancelled() => return,
        };
        match stream {
            Ok(stream) => {
                tokio::spawn(session(stream, host.clone(), stop.child_token()));
            }
            Err(error) => {
                SERVER.lock().error = Some(format!("Agent connection failed: {error}"));
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
        }
    }
}

/// One agent's connection, until it hangs up or the server stops.
async fn session(stream: impl ipc::Stream, host: Arc<dyn Host>, stop: CancellationToken) {
    use rmcp::ServiceExt;
    let id = {
        let mut server = SERVER.lock();
        server.next_connection += 1;
        let id = server.next_connection;
        server.connected.push((id, None));
        id
    };
    let handler = Handler {
        host,
        connection: id,
    };
    if let Ok(running) = handler.serve_with_ct(stream, stop).await {
        let _ = running.waiting().await;
    }
    SERVER.lock().connected.retain(|(c, _)| *c != id);
}

/// Kept short: some clients repeat it in front of every tool they look up. The rules come
/// once, from `template`.
const INSTRUCTIONS: &str = "AeroEdits edits screen-recorded tutorials, on the project the user \
has open. For an edit of a video, call `template` first and follow it: it has the rules, the \
steps, what to ask and the export steps; never export before the user says go. The app's \
actions (scan, transcribe, silence, fillers, zooms, chapters, export, ...) do the work, each as \
one undo step answering in a line or two. Times are microseconds on the edited timeline. Pass \
the `project` id from `status` on every call that changes something.";

#[derive(Clone)]
struct Handler {
    host: Arc<dyn Host>,
    connection: u64,
}

impl rmcp::ServerHandler for Handler {
    fn get_info(&self) -> rmcp::model::ServerConfig {
        server_info()
    }

    async fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::ListToolsResult, rmcp::ErrorData> {
        self.note_client(&context);
        Ok(rmcp::model::ListToolsResult {
            tools: tool_list(),
            ..Default::default()
        })
    }

    async fn list_prompts(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::ListPromptsResult, rmcp::ErrorData> {
        self.note_client(&context);
        Ok(rmcp::model::ListPromptsResult {
            prompts: skill::prompt_list(),
            ..Default::default()
        })
    }

    async fn get_prompt(
        &self,
        request: rmcp::model::GetPromptRequestParams,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::GetPromptResponse, rmcp::ErrorData> {
        self.note_client(&context);
        skill::prompt_get(
            &self.host.config_dir(),
            &request.name,
            request.arguments.as_ref(),
        )
        .map(Into::into)
        .map_err(|error| rmcp::ErrorData::invalid_params(error, None))
    }

    async fn call_tool(
        &self,
        request: rmcp::model::CallToolRequestParams,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::CallToolResponse, rmcp::ErrorData> {
        use rmcp::model::{CallToolResult, ContentBlock};
        let client = self.note_client(&context);
        let Some(tool) = tools::find(&request.name) else {
            return Err(rmcp::ErrorData::invalid_params(
                format!("No tool named {}", request.name),
                None,
            ));
        };
        let args = request
            .arguments
            .map(serde_json::Value::Object)
            .unwrap_or(serde_json::Value::Null);
        let host = self.host.clone();
        let started = std::time::Instant::now();
        let agent = client.clone().unwrap_or_else(|| "agent".into());
        let call = {
            let mut server = SERVER.lock();
            server.next_call += 1;
            let id = server.next_call;
            server.working.push(Working {
                id,
                client: client.clone(),
                tool: tool.name.to_string(),
                fraction: None,
                message: None,
            });
            id
        };
        self.host.activity_changed();
        // Progress goes to the agent in order, through one task, when it asked for it.
        let (progress_tx, mut progress_rx) =
            tokio::sync::mpsc::unbounded_channel::<(f64, String)>();
        let token = context.meta.get_progress_token();
        let peer = context.peer.clone();
        let forward = tokio::spawn(async move {
            while let Some((fraction, message)) = progress_rx.recv().await {
                if let Some(token) = &token {
                    let mut param =
                        rmcp::model::ProgressNotificationParam::new(token.clone(), fraction);
                    param.total = Some(1.0);
                    param.message = Some(message);
                    let _ = peer.notify_progress(param).await;
                }
            }
        });
        let outcome = tokio::task::spawn_blocking(move || {
            let progress_host = host.clone();
            let report = move |fraction: f64, message: &str| {
                if let Some(entry) = SERVER.lock().working.iter_mut().find(|w| w.id == call) {
                    entry.fraction = Some(fraction);
                    entry.message = Some(message.to_string());
                }
                progress_host.activity_changed();
                let _ = progress_tx.send((fraction, message.to_string()));
            };
            let mut ctx = tools::Ctx::new(host.as_ref(), agent, host.autopilot())
                .with_progress(Box::new(report));
            let out = tools::call(tool, &mut ctx, args);
            if ctx.project_was_changed() {
                host.project_changed();
            }
            if ctx.proposals_were_changed() {
                host.proposals_changed();
            }
            out
        })
        .await
        .unwrap_or_else(|e| Err(format!("The tool stopped: {e}")));
        let _ = forward.await;
        SERVER.lock().working.retain(|w| w.id != call);
        self.host.activity_changed();
        record(Activity {
            at: chrono::Local::now().format("%H:%M:%S").to_string(),
            client,
            tool: tool.name.to_string(),
            ok: outcome.is_ok(),
            millis: started.elapsed().as_millis() as u64,
            error: outcome.as_ref().err().cloned(),
        });
        let result = match outcome {
            Ok(outputs) => CallToolResult::success(
                outputs
                    .into_iter()
                    .map(|output| match output {
                        tools::Output::Text(text) => ContentBlock::text(text),
                        tools::Output::Png(bytes) => {
                            use base64::Engine;
                            ContentBlock::image(
                                base64::engine::general_purpose::STANDARD.encode(bytes),
                                "image/png",
                            )
                        }
                    })
                    .collect(),
            ),
            Err(error) => CallToolResult::error(vec![ContentBlock::text(error)]),
        };
        Ok(result.into())
    }
}

impl Handler {
    /// The agent's name from its handshake, kept for the connected list.
    fn note_client(
        &self,
        context: &rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Option<String> {
        let name = context
            .peer
            .peer_info()
            .map(|info| info.client_info.name.clone())?;
        let mut server = SERVER.lock();
        if let Some(entry) = server
            .connected
            .iter_mut()
            .find(|(c, _)| *c == self.connection)
        {
            entry.1 = Some(name.clone());
        }
        Some(name)
    }
}

/// What the server says about itself in the handshake; the bridge says the same while the app
/// is closed.
pub(crate) fn server_info() -> rmcp::model::ServerConfig {
    use rmcp::model::{Implementation, ServerCapabilities, ServerConfig};
    ServerConfig::new(
        ServerCapabilities::builder()
            .enable_tools()
            .enable_prompts()
            .build(),
    )
    .with_server_info(Implementation::new("aeroedits", env!("CARGO_PKG_VERSION")))
    .with_instructions(INSTRUCTIONS)
}

/// Every tool as `tools/list` lists it: all but those behind `more`.
pub(crate) fn tool_list() -> Vec<rmcp::model::Tool> {
    tools::all()
        .iter()
        .filter(|tool| !tools::MORE.contains(&tool.name))
        .map(tool_entry)
        .collect()
}

pub(crate) fn tool_entry(tool: &tools::ToolDef) -> rmcp::model::Tool {
    let mut schema = match (tool.schema)() {
        serde_json::Value::Object(map) => map,
        _ => Default::default(),
    };
    lean(&mut schema);
    if tools::takes_project(tool) {
        bind_to_project(&mut schema, tools::NEEDS_PROJECT.contains(&tool.name));
    }
    let annotations = rmcp::model::ToolAnnotations::new()
        .read_only(tool.read_only)
        .destructive(tools::destructive(tool.name));
    rmcp::model::Tool::new(tool.name, tool.description, Arc::new(schema))
        .with_annotations(annotations)
}

/// What agents are sent of a tool's schema, without what changes nothing about a call: the
/// `$schema` and `title` labels, `format` hints, `default: null`; and "a string or null" said
/// the short way (`type: ["string", "null"]`, the same thing). Agents read every tool's schema
/// on each connection, so this is paid again and again. The `aeroedits do --help` text keeps
/// the full schema.
fn lean(schema: &mut serde_json::Map<String, serde_json::Value>) {
    use serde_json::Value;
    schema.remove("$schema");
    schema.remove("title");
    schema.remove("format");
    if schema.get("default") == Some(&Value::Null) {
        schema.remove("default");
    }
    for (key, value) in schema.iter_mut() {
        match (key.as_str(), value) {
            // Maps of names to schemas: the names stay, each schema is made lean.
            ("properties" | "$defs" | "definitions", Value::Object(named)) => {
                for value in named.values_mut() {
                    if let Value::Object(inner) = value {
                        lean(inner);
                    }
                }
            }
            ("items" | "additionalProperties" | "not", Value::Object(inner)) => lean(inner),
            ("anyOf" | "oneOf" | "allOf" | "prefixItems", Value::Array(list)) => {
                for value in list {
                    if let Value::Object(inner) = value {
                        lean(inner);
                    }
                }
            }
            _ => {}
        }
    }
    // Then: anyOf [{type: X}, {type: null}] → type [X, null].
    if let Some(Value::Array(options)) = schema.get("anyOf") {
        let null = serde_json::json!({ "type": "null" });
        if options.len() == 2 && options.contains(&null) {
            let other = options.iter().find(|o| **o != null).cloned();
            if let Some(Value::Object(other)) = other {
                if let (1, Some(Value::String(kind))) = (other.len(), other.get("type")) {
                    let kind = kind.clone();
                    schema.remove("anyOf");
                    schema.insert("type".into(), serde_json::json!([kind, "null"]));
                }
            }
        }
    }
}

/// Adds the `project` argument [`tools::call`] takes out before a tool sees its arguments.
fn bind_to_project(schema: &mut serde_json::Map<String, serde_json::Value>, required: bool) {
    use serde_json::{json, Value};
    let properties = schema
        .entry("properties")
        .or_insert_with(|| json!({}))
        .as_object_mut();
    if let Some(properties) = properties {
        properties.insert(
            "project".into(),
            json!({
                "type": "string",
                "description": "The open project's id, from status.",
            }),
        );
    }
    if required {
        let list = schema.entry("required").or_insert_with(|| json!([]));
        if let Value::Array(list) = list {
            list.push(json!("project"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands;
    use serde_json::{json, Value};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    struct TestHost(
        AppState,
        bool,
        std::sync::atomic::AtomicUsize,
        crate::commands::transcript::TranscriptState,
    );

    impl TestHost {
        fn new(state: AppState) -> Self {
            Self(state, false, Default::default(), Default::default())
        }
    }

    impl Host for TestHost {
        fn state(&self) -> &AppState {
            &self.0
        }

        fn transcripts(&self) -> &crate::commands::transcript::TranscriptState {
            &self.3
        }

        fn ai_review_ready(&self) -> bool {
            false
        }

        fn local_review_ready(&self) -> bool {
            false
        }

        fn autopilot(&self) -> bool {
            self.1
        }

        fn proposals_changed(&self) {
            self.2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }

    /// A project with one sound on the timeline and a transcript of it.
    fn project(dir: &Path) -> Arc<TestHost> {
        project_with(dir, false)
    }

    fn project_with(dir: &Path, autopilot: bool) -> Arc<TestHost> {
        let wav = dir.join("voice.wav");
        let samples = vec![0i16; 48_000 * 2];
        fs::write(
            &wav,
            crate::fixtures::generate_pcm16_wav(48_000, 1, &samples),
        )
        .unwrap();
        let folder = crate::project::folder::create_project_folder(dir, "Lesson", None).unwrap();
        let mut host = TestHost::new(AppState::new());
        host.1 = autopilot;
        let host = Arc::new(host);
        let state = host.state();
        let opened = commands::open_project_impl(state, folder.to_string_lossy().into()).unwrap();
        let opened = commands::project_media_import_impl(
            state,
            opened.project_handle.clone(),
            opened.revision,
            vec![wav.to_string_lossy().into()],
        )
        .unwrap();
        let asset = opened.assets[0].clone();
        commands::project_sequence_edit_impl(
            state,
            opened.project_handle.clone(),
            opened.revision,
            serde_json::from_value(json!({ "kind": "placeAsset", "assetId": asset.id, "atUs": 0 }))
                .unwrap(),
            None,
        )
        .unwrap();
        let key = format!("{}.{}", asset.id, asset.streams[0].id);
        let words = [("Hello", 0, 300), ("there.", 320, 700), ("Next", 900, 1200)];
        let transcript = crate::transcript::Transcript {
            schema_version: crate::transcript::TRANSCRIPT_SCHEMA_VERSION,
            track_id: key.clone(),
            provider: crate::transcript::ProviderKind::Parakeet,
            model: "test".into(),
            language: None,
            created_at: "2026-10-13T00:00:00Z".into(),
            words: words
                .iter()
                .enumerate()
                .map(
                    |(i, (text, start, end))| crate::transcript::TranscriptWord {
                        id: format!("w-{i}"),
                        text: (*text).into(),
                        kind: crate::transcript::WordKind::Word,
                        source_start_us: start * 1000,
                        source_end_us: end * 1000,
                        confidence: None,
                        speaker: None,
                    },
                )
                .collect(),
            dismissed_suggestions: Vec::new(),
            ai_suggestions: Vec::new(),
            ai_model: None,
            caption_marks: Default::default(),
        };
        crate::transcript::store::save_transcript(&folder, &transcript).unwrap();
        host
    }

    /// An endpoint of this test's own, so tests and a running app never meet.
    fn test_endpoint() -> ipc::Endpoint {
        let id = uuid::Uuid::new_v4().simple().to_string();
        #[cfg(windows)]
        return ipc::Endpoint(format!(r"\\.\pipe\aeroedits-mcp-test-{id}"));
        #[cfg(unix)]
        return ipc::Endpoint(
            std::env::temp_dir()
                .join(format!("aeroedits-{}", &id[..8]))
                .join("mcp.sock")
                .to_string_lossy()
                .into_owned(),
        );
    }

    /// Sends `messages` through the bridge as an agent would and returns `expect` replies.
    async fn talk(endpoint: &ipc::Endpoint, messages: &[Value], expect: usize) -> Vec<Value> {
        let stream = ipc::connect(endpoint).await.unwrap();
        let (mut agent_out, bridge_in) = tokio::io::duplex(1 << 20);
        let (bridge_out, agent_in) = tokio::io::duplex(1 << 20);
        let pump = tokio::spawn(bridge::pump(stream, bridge_in, bridge_out));
        for message in messages {
            agent_out
                .write_all(format!("{message}\n").as_bytes())
                .await
                .unwrap();
        }
        let mut lines = BufReader::new(agent_in).lines();
        let mut replies = Vec::new();
        while replies.len() < expect {
            let line = tokio::time::timeout(std::time::Duration::from_secs(20), lines.next_line())
                .await
                .expect("a reply in time")
                .unwrap()
                .expect("the pipe stays open");
            replies.push(serde_json::from_str(&line).unwrap());
        }
        drop(agent_out);
        let _ = pump.await;
        replies
    }

    #[test]
    fn agents_reach_the_open_project_through_the_bridge_and_pipe() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let host = project(dir.path());
        let endpoint = test_endpoint();
        let running = serve(host, endpoint.clone()).unwrap();
        // A second server on the same name is refused, so nothing can take it over.
        assert!(serve(Arc::new(TestHost::new(AppState::new())), endpoint.clone()).is_err());

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let replies = runtime.block_on(talk(
            &endpoint,
            &[
                json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
                    "protocolVersion": "2025-06-18", "capabilities": {},
                    "clientInfo": {"name": "pipe-test", "version": "1"}}}),
                json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
                json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
                json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call",
                    "params": {"name": "transcript_get", "arguments": {}}}),
                json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call",
                    "params": {"name": "frame_render", "arguments": {"atUs": "soon"}}}),
                json!({"jsonrpc": "2.0", "id": 5, "method": "prompts/list"}),
                json!({"jsonrpc": "2.0", "id": 6, "method": "prompts/get",
                    "params": {"name": "tutorial"}}),
            ],
            6,
        ));
        let by_id = |id: u64| replies.iter().find(|r| r["id"] == id).unwrap().clone();
        assert_eq!(by_id(1)["result"]["serverInfo"]["name"], "aeroedits");
        let names: Vec<String> = by_id(2)["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap().to_string())
            .collect();
        assert!(names.iter().any(|n| n == "project_info"), "{names:?}");
        let text = by_id(3)["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(text.contains("w-0..w-1 Hello there."), "{text}");
        assert_eq!(by_id(4)["result"]["isError"], true);
        assert_eq!(by_id(5)["result"]["prompts"][0]["name"], "tutorial");
        let prompt = by_id(6)["result"]["messages"][0]["content"]["text"].clone();
        assert!(
            prompt
                .as_str()
                .unwrap()
                .starts_with("Edit the project open in AeroEdits"),
            "{prompt}"
        );
        let activity = SERVER.lock().activity.clone();
        assert!(activity.iter().any(|a| a.tool == "transcript_get"
            && a.ok
            && a.client.as_deref() == Some("pipe-test")));

        running.stop.cancel();
    }

    fn handshake(name: &str) -> Vec<Value> {
        vec![
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
                "protocolVersion": "2025-06-18", "capabilities": {},
                "clientInfo": {"name": name, "version": "1"}}}),
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        ]
    }

    fn call(id: u64, name: &str, arguments: Value) -> Value {
        json!({"jsonrpc": "2.0", "id": id, "method": "tools/call",
            "params": {"name": name, "arguments": arguments}})
    }

    fn reply_text(replies: &[Value], id: u64) -> String {
        let reply = replies.iter().find(|r| r["id"] == id).unwrap();
        reply["result"]["content"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|c| c["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn project_id(host: &TestHost) -> String {
        host.0
            .opened_project
            .lock()
            .as_ref()
            .unwrap()
            .summary
            .project_handle
            .clone()
    }

    fn revision(host: &TestHost) -> u64 {
        host.0
            .opened_project
            .lock()
            .as_ref()
            .unwrap()
            .document()
            .revision
    }

    #[test]
    fn an_agents_cut_proposal_goes_through_the_pipe_and_the_user_accepts_it() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let host = project(dir.path());
        let endpoint = test_endpoint();
        let running = serve(host.clone(), endpoint.clone()).unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let start = revision(&host);

        let mut messages = handshake("claude-code");
        messages.push(call(
            2,
            "propose_cuts",
            json!({
                "project": project_id(&host),
                "expectedRevision": start,
                "reason": "Remove the false start",
                "cuts": [{ "fromWord": "w-0", "toWord": "w-1", "reason": "restarted" }],
            }),
        ));
        let replies = runtime.block_on(talk(&endpoint, &messages, 2));
        let proposed = reply_text(&replies, 2);
        assert!(
            proposed.starts_with("Proposed b-1. It waits for the user"),
            "{proposed}"
        );
        assert!(
            proposed.contains("c-1 w-0..w-1 \"Hello there.\""),
            "{proposed}"
        );
        assert_eq!(revision(&host), start, "proposing changes nothing");

        // Requests on one connection run side by side, so each step waits for the one before.
        let mut messages = handshake("claude-code");
        messages.push(call(3, "proposals_list", json!({ "pendingOnly": true })));
        messages.push(call(
            4,
            "timeline_view",
            json!({ "fromUs": 0, "toUs": 2_000_000, "width": 800 }),
        ));
        messages.push(call(5, "telemetry_get", json!({})));
        let replies = runtime.block_on(talk(&endpoint, &messages, 4));
        let listed = reply_text(&replies, 3);
        assert!(
            listed.contains("\"yours\": true") && listed.contains("\"status\": \"pending\""),
            "{listed}"
        );
        let view = replies.iter().find(|r| r["id"] == 4).unwrap();
        assert_eq!(view["result"]["content"][0]["type"], "image", "{view}");
        assert!(
            reply_text(&replies, 4).contains("1 pending cut shaded"),
            "{view}"
        );
        assert!(reply_text(&replies, 5).contains("Zooms in the edit (0)"));
        assert!(host.2.load(std::sync::atomic::Ordering::SeqCst) >= 1);

        // The user accepts it in the Agent panel.
        let handle = host
            .0
            .opened_project
            .lock()
            .as_ref()
            .unwrap()
            .summary
            .project_handle
            .clone();
        let outcome = commands::proposals::proposal_accept_impl(
            &host.0,
            handle,
            start,
            "b-1".into(),
            None,
            true,
        )
        .unwrap();
        assert_eq!(outcome.project.unwrap().revision, start + 1);

        let mut messages = handshake("claude-code");
        messages.push(call(2, "proposals_list", json!({})));
        messages.push(call(3, "transcript_get", json!({})));
        let replies = runtime.block_on(talk(&endpoint, &messages, 3));
        assert!(reply_text(&replies, 2).contains("\"status\": \"accepted\""));
        let transcript = reply_text(&replies, 3);
        assert!(
            transcript.contains("2 cut out") && !transcript.contains("Hello"),
            "{transcript}"
        );

        let mut messages = handshake("claude-code");
        messages.push(call(
            2,
            "undo_batch",
            json!({ "project": project_id(&host), "batchId": "b-1", "expectedRevision": start + 1 }),
        ));
        let replies = runtime.block_on(talk(&endpoint, &messages, 2));
        assert!(
            reply_text(&replies, 2).starts_with("Undid b-1"),
            "{}",
            reply_text(&replies, 2)
        );
        assert_eq!(revision(&host), start + 2);

        running.stop.cancel();
    }

    #[test]
    fn under_autopilot_a_proposal_is_accepted_as_it_arrives() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let host = project_with(dir.path(), true);
        let endpoint = test_endpoint();
        let running = serve(host.clone(), endpoint.clone()).unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let start = revision(&host);
        let mut messages = handshake("codex");
        messages.push(call(
            2,
            "propose_edit",
            json!({
                "project": project_id(&host),
                "expectedRevision": start,
                "reason": "A track for the lower third",
                "edits": [{ "kind": "addTrack", "trackKind": "video" }],
            }),
        ));
        messages.push(call(
            3,
            "propose_edit",
            json!({
                "project": project_id(&host),
                "expectedRevision": start,
                "reason": "stale",
                "edits": [{ "kind": "teleport" }],
            }),
        ));
        let replies = runtime.block_on(talk(&endpoint, &messages, 3));
        let accepted = reply_text(&replies, 2);
        assert!(accepted.contains("autopilot accepted it"), "{accepted}");
        assert_eq!(revision(&host), start + 1);
        let refused = replies.iter().find(|r| r["id"] == 3).unwrap();
        assert_eq!(refused["result"]["isError"], true);
        assert!(reply_text(&replies, 3).contains("Edit 1 is not a timeline edit"));
        running.stop.cancel();
    }

    #[test]
    fn the_cli_runs_an_action_over_the_pipe_with_progress() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, _) = tests_support::lesson(dir.path());
        let host = Arc::new(host);
        let endpoint = test_endpoint();
        let running = serve(host.clone(), endpoint.clone()).unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let ask = |args: &[&str]| {
            let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
            let (mut out, mut err) = (Vec::new(), Vec::new());
            let code = runtime.block_on(async {
                let stream = ipc::connect(&endpoint).await.unwrap();
                cli::session(
                    stream,
                    cli::parse(&args).unwrap(),
                    "cli-test",
                    &mut out,
                    &mut err,
                )
                .await
            });
            (
                code,
                String::from_utf8(out).unwrap(),
                String::from_utf8(err).unwrap(),
            )
        };

        let (code, out, _) = ask(&[]);
        assert_eq!(code, 0);
        assert!(
            out.contains("  silence ") && out.contains("  transcript_get "),
            "{out}"
        );
        let (code, out, _) = ask(&["silence", "--help"]);
        assert_eq!(code, 0);
        assert!(
            out.contains("--preset <gentle|balanced|tight|aggressive>"),
            "{out}"
        );

        let revision = host
            .state
            .opened_project
            .lock()
            .as_ref()
            .unwrap()
            .document()
            .revision;
        let (code, out, err) = ask(&["silence", "--preset", "tight"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(out.starts_with("Found 3 pauses"), "{out}");
        assert!(
            err.contains("Finding silence in"),
            "progress goes to stderr: {err}"
        );
        let after = host
            .state
            .opened_project
            .lock()
            .as_ref()
            .unwrap()
            .document()
            .revision;
        assert_eq!(after, revision + 1);
        assert!(SERVER
            .lock()
            .activity
            .iter()
            .any(|a| a.tool == "silence" && a.client.as_deref() == Some("cli-test")));

        let (code, _, err) = ask(&["silence", "--loudness", "3"]);
        assert_eq!(code, 2);
        assert!(err.contains("Unknown option --loudness"), "{err}");
        let (code, out, _) = ask(&["fillers", "--stream", "nope.mic"]);
        assert_eq!(code, 1);
        assert!(out.contains("No sound nope.mic"), "{out}");
        running.stop.cancel();
    }

    #[test]
    fn the_bridge_keeps_the_tools_and_reconnects_whenever_the_app_comes_and_goes() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let host = project(dir.path());
        let endpoint = test_endpoint();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        // The agent's app starts the bridge once and keeps it, as the Claude app does.
        let (mut agent_out, bridge_in) = tokio::io::duplex(1 << 20);
        let (bridge_out, agent_in) = tokio::io::duplex(1 << 20);
        runtime.spawn(bridge::relay(bridge_in, bridge_out, endpoint.clone()));
        let mut lines = BufReader::new(agent_in).lines();
        let mut ask = |message: Value| {
            runtime.block_on(async {
                agent_out
                    .write_all(format!("{message}\n").as_bytes())
                    .await
                    .unwrap();
                let line =
                    tokio::time::timeout(std::time::Duration::from_secs(20), lines.next_line())
                        .await
                        .expect("a reply in time")
                        .unwrap()
                        .unwrap();
                serde_json::from_str::<Value>(&line).unwrap()
            })
        };
        let text = |reply: &Value| {
            reply["result"]["content"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        };
        let call = |id: u64| {
            json!({"jsonrpc": "2.0", "id": id, "method": "tools/call",
            "params": {"name": "transcript_get", "arguments": {}}})
        };

        // A client that asks the newer discovery first is told it is unknown, so it does the
        // handshake (with its name) instead.
        let discover = ask(
            json!({"jsonrpc": "2.0", "id": "d-1", "method": "server/discover",
            "params": {"_meta": {"io.modelcontextprotocol/clientInfo": {"name": "claude-ai"}}}}),
        );
        assert_eq!(discover["error"]["code"], -32601, "{discover}");
        // The app is closed: the handshake and the tools still come, from the bridge.
        let hello = ask(
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
            "protocolVersion": "2025-06-18", "capabilities": {},
            "clientInfo": {"name": "claude-ai", "version": "1"}}}),
        );
        assert_eq!(hello["result"]["serverInfo"]["name"], "aeroedits");
        assert_eq!(hello["result"]["protocolVersion"], "2025-06-18");
        // Short, since some clients repeat it before every tool; the rules come from `template`.
        let instructions = hello["result"]["instructions"].as_str().unwrap();
        assert!(instructions.contains("never export before the user says go"));
        assert!(instructions.len() < 700, "{}", instructions.len());
        let tools = ask(json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}));
        let names: Vec<&str> = tools["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|t| t["name"].as_str())
            .collect();
        assert!(
            names.contains(&"silence") && names.contains(&"transcript_get"),
            "{names:?}"
        );
        // There is no one-call cleanup: agents run and report one pass at a time.
        assert!(!names.contains(&"cleanup"), "{names:?}");
        assert!(
            names.contains(&"template") && names.contains(&"scan"),
            "{names:?}"
        );
        assert!(
            hello["result"]["capabilities"]["prompts"].is_object(),
            "{hello}"
        );
        assert!(hello["result"]["instructions"]
            .as_str()
            .unwrap()
            .contains("call `template` first"));
        // Templates need no app either.
        let prompts = ask(json!({"jsonrpc": "2.0", "id": 20, "method": "prompts/list"}));
        assert_eq!(
            prompts["result"]["prompts"][0]["title"], "Tutorial edit",
            "{prompts}"
        );
        let prompt = ask(
            json!({"jsonrpc": "2.0", "id": 21, "method": "prompts/get", "params": {
            "name": "tutorial", "arguments": {"recording": r"C:\Rec\Lesson.aero"}}}),
        );
        let message = prompt["result"]["messages"][0]["content"]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(
            message.starts_with(r"Edit the recording C:\Rec\Lesson.aero in AeroEdits"),
            "{prompt}"
        );
        assert!(message.contains("# Template: tutorial"), "{prompt}");
        let missing = ask(
            json!({"jsonrpc": "2.0", "id": 22, "method": "prompts/get", "params": {"name": "vlog"}}),
        );
        assert_eq!(missing["error"]["code"], -32602, "{missing}");
        let closed = ask(call(3));
        assert_eq!(closed["result"]["isError"], true);
        assert_eq!(text(&closed), bridge::NOT_RUNNING);

        // The app opens: the next call reaches it, under the agent's name.
        let running = serve(host.clone(), endpoint.clone()).unwrap();
        let answered = ask(call(4));
        assert!(
            text(&answered).contains("w-0..w-1 Hello there."),
            "{answered}"
        );
        assert!(SERVER
            .lock()
            .activity
            .iter()
            .any(|a| a.tool == "transcript_get" && a.client.as_deref() == Some("claude-ai")));

        // It closes (a reinstall), and opens again: the same bridge carries on.
        running.stop.cancel();
        std::thread::sleep(std::time::Duration::from_millis(500));
        let gone = ask(call(5));
        assert_eq!(gone["result"]["isError"], true, "{gone}");
        let running = serve(host.clone(), endpoint.clone()).unwrap();
        let back = ask(call(6));
        assert!(text(&back).contains("Hello there."), "{back}");
        running.stop.cancel();
    }

    #[test]
    fn lean_schemas_keep_what_a_call_can_be() {
        let mut schema = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "title": "Args",
            "type": "object",
            "properties": {
                // Parameters named like schema words stay.
                "title": { "type": "string", "description": "kept" },
                "format": { "anyOf": [{ "type": "integer", "format": "uint64" }, { "type": "null" }],
                            "default": null, "minimum": 0 },
                "kind": { "anyOf": [{ "$ref": "#/$defs/Kind" }, { "type": "null" }], "default": null },
            },
            "required": ["title"],
            "$defs": { "Kind": { "title": "Kind", "enum": ["a", "b"], "default": "a" } },
        });
        let Value::Object(ref mut map) = schema else {
            unreachable!()
        };
        lean(map);
        assert_eq!(
            schema,
            json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "kept" },
                    "format": { "type": ["integer", "null"], "minimum": 0 },
                    "kind": { "anyOf": [{ "$ref": "#/$defs/Kind" }, { "type": "null" }] },
                },
                "required": ["title"],
                "$defs": { "Kind": { "enum": ["a", "b"], "default": "a" } },
            })
        );
        let mut simple =
            json!({ "anyOf": [{ "type": "string" }, { "type": "null" }], "default": null });
        let Value::Object(ref mut map) = simple else {
            unreachable!()
        };
        lean(map);
        assert_eq!(simple, json!({ "type": ["string", "null"] }));
    }

    #[test]
    fn rarely_needed_tools_are_reached_through_more() {
        let listed: Vec<String> = tool_list().iter().map(|t| t.name.to_string()).collect();
        let more = tools::find("more").unwrap();
        for name in tools::MORE {
            assert!(tools::find(name).is_some(), "{name} is a tool");
            assert!(!listed.contains(&name.to_string()), "{name} is listed");
            assert!(
                more.description.contains(name),
                "`more` doesn't name {name}"
            );
        }
        assert!(listed.contains(&"more".to_string()));
        let host = tests_support::BareHost::default();
        let mut ctx = tools::Ctx::new(&host, "agent", false);
        let mut run = |args: serde_json::Value| {
            tools::call(more, &mut ctx, args).map(|out| match &out[0] {
                tools::Output::Text(text) => text.clone(),
                tools::Output::Png(_) => String::new(),
            })
        };
        // Without `args`: what the tool does and takes.
        let help = run(serde_json::json!({ "tool": "template_save" })).unwrap();
        assert!(
            help.starts_with("template_save: Saves") && help.contains("\"answers\""),
            "{help}"
        );
        // With them: it runs as if called directly.
        let saved = run(serde_json::json!({ "tool": "template_save",
            "args": { "name": "tutorial", "answers": { "silence": "gentle" } } }))
        .unwrap();
        assert!(
            saved.starts_with("Saved the answers for tutorial"),
            "{saved}"
        );
        // Only the tools behind it.
        assert!(run(serde_json::json!({ "tool": "status", "args": {} }))
            .unwrap_err()
            .contains("they are: redo"));
    }

    #[test]
    fn the_tool_list_stays_small() {
        // Agents read it on every connection: 84,414 characters before 23 Oct 2026, 72,722
        // after, 53,828 with the rarely needed tools behind `more`.
        let list = tool_list();
        if std::env::var("TOOL_SIZES").is_ok() {
            for tool in &list {
                eprintln!(
                    "TOOL {} {}",
                    tool.name,
                    serde_json::to_string(tool).unwrap().len()
                );
            }
        }
        let size = serde_json::to_string(&list).unwrap().len();
        assert!(size < 60_000, "the tool list grew to {size} characters");
    }
}
