//! `aeroedits --mcp-stdio`: what an agent starts to reach AeroEdits. The agent's app (the
//! Claude app, Codex, Claude Code, ...) starts it once and keeps it, whether AeroEdits is open
//! or not, so it never depends on which was started first:
//!
//! - It answers the handshake and the tool list itself (it is the same program as the app, so
//!   it knows the tools), and the agent always sees AeroEdits' tools.
//! - It reaches the running app over its private pipe ([`super::ipc`]) only for a tool call,
//!   and connects again after the app was closed or reinstalled.
//! - With the app closed (or agents turned off), that call answers what to do.
//!
//! No port, no token, no settings to hold.
use serde_json::{json, Value};
use std::collections::HashSet;
use tokio::io::{
    AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, Lines,
    ReadHalf, WriteHalf,
};

pub(super) const NOT_RUNNING: &str = "AeroEdits is not running, or AI agents are off. Open \
AeroEdits and turn on File > AI agents > Allow agents to connect, then try again.";

const CLOSED_DURING_CALL: &str = "AeroEdits closed during this call. Open it and try again; \
anything it finished before closing was saved.";

/// The id the bridge's own handshake with the app goes under.
const HANDSHAKE_ID: &str = "aeroedits-bridge-handshake";

/// Runs until the agent hangs up. Returns the process exit code.
pub fn run() -> i32 {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("aeroedits --mcp-stdio: {error}");
            return 1;
        }
    };
    runtime.block_on(async {
        let endpoint = match super::ipc::default_endpoint() {
            Ok(endpoint) => endpoint,
            Err(error) => {
                eprintln!("aeroedits --mcp-stdio: {error}");
                return 1;
            }
        };
        match relay(tokio::io::stdin(), tokio::io::stdout(), endpoint).await {
            Ok(()) => 0,
            Err(error) => {
                eprintln!("aeroedits --mcp-stdio: {error}");
                1
            }
        }
    })
}

/// The connection to the running app, while there is one.
struct App<S> {
    write: WriteHalf<S>,
    lines: Lines<BufReader<ReadHalf<S>>>,
    /// Requests forwarded and not yet answered, by their id as JSON text.
    pending: HashSet<String>,
}

fn reply(id: &Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

/// A tool call's answer that the agent shows as the tool's error, so it can tell the user.
fn tool_error(id: &Value, message: &str) -> Value {
    reply(
        id,
        json!({ "content": [{ "type": "text", "text": message }], "isError": true }),
    )
}

/// The handshake answer, in the protocol version the agent asked for when it is one there is
/// a handshake in.
fn initialize_result(params: &Value) -> Value {
    use rmcp::model::ProtocolVersion;
    let asked = params.get("protocolVersion").and_then(Value::as_str);
    let version = ProtocolVersion::KNOWN_VERSIONS
        .iter()
        .find(|v| Some(v.as_str()) == asked && v.has_initialize())
        .cloned()
        .unwrap_or(ProtocolVersion::LATEST_WITH_INITIALIZE);
    let mut result = serde_json::to_value(super::server_info()).unwrap_or_default();
    result["protocolVersion"] = json!(version.as_str());
    result
}

async fn write_line(
    output: &mut (impl AsyncWrite + Unpin),
    message: &Value,
) -> std::io::Result<()> {
    output.write_all(format!("{message}\n").as_bytes()).await?;
    output.flush().await
}

/// Connects to the app and does the handshake on the agent's behalf, under the agent's name.
async fn connect<S: super::ipc::Stream>(stream: S, hello: &Value) -> Result<App<S>, String> {
    let (read, mut write) = tokio::io::split(stream);
    let mut lines = BufReader::new(read).lines();
    let client_info = hello
        .get("clientInfo")
        .cloned()
        .unwrap_or_else(|| json!({ "name": "agent", "version": "0" }));
    let initialize = json!({
        "jsonrpc": "2.0", "id": HANDSHAKE_ID, "method": "initialize",
        "params": {
            "protocolVersion": rmcp::model::ProtocolVersion::LATEST_WITH_INITIALIZE.as_str(),
            "capabilities": {},
            "clientInfo": client_info,
        }
    });
    write_line(&mut write, &initialize)
        .await
        .map_err(|e| e.to_string())?;
    let answered = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while let Some(line) = lines.next_line().await.map_err(|e| e.to_string())? {
            if serde_json::from_str::<Value>(&line)
                .is_ok_and(|message| message.get("id") == Some(&json!(HANDSHAKE_ID)))
            {
                return Ok(());
            }
        }
        Err::<(), String>(NOT_RUNNING.into())
    })
    .await;
    match answered {
        Ok(Ok(())) => {}
        Ok(Err(error)) => return Err(error),
        Err(_) => return Err("AeroEdits did not answer. Is it busy? Try again.".into()),
    }
    let initialized = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
    write_line(&mut write, &initialized)
        .await
        .map_err(|e| e.to_string())?;
    Ok(App {
        write,
        lines,
        pending: HashSet::new(),
    })
}

/// Serves the agent on `input` and `output` until it hangs up, reaching the app at `endpoint`
/// whenever a call needs it.
pub async fn relay(
    input: impl AsyncRead + Unpin,
    mut output: impl AsyncWrite + Unpin,
    endpoint: super::ipc::Endpoint,
) -> std::io::Result<()> {
    let mut agent = BufReader::new(input).lines();
    // The agent's handshake, to repeat to the app under its name.
    let mut hello = Value::Null;
    let mut app = None;
    loop {
        tokio::select! {
            line = agent.next_line() => {
                let Some(line) = line? else {
                    return Ok(());
                };
                let Ok(message) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                let method = message.get("method").and_then(Value::as_str).unwrap_or_default();
                let id = message.get("id").cloned();
                match (method, &id) {
                    ("initialize", Some(id)) => {
                        hello = message.get("params").cloned().unwrap_or_default();
                        write_line(&mut output, &reply(id, initialize_result(&hello))).await?;
                    }
                    ("notifications/initialized", None) => {}
                    ("ping", Some(id)) => write_line(&mut output, &reply(id, json!({}))).await?,
                    ("tools/list", Some(id)) => {
                        let tools = serde_json::to_value(super::tool_list()).unwrap_or_default();
                        write_line(&mut output, &reply(id, json!({ "tools": tools }))).await?;
                    }
                    // Templates are the app's text and the user's own answers: no app needed.
                    ("prompts/list", Some(id)) => {
                        let prompts = serde_json::to_value(super::skill::prompt_list()).unwrap_or_default();
                        write_line(&mut output, &reply(id, json!({ "prompts": prompts }))).await?;
                    }
                    ("prompts/get", Some(id)) => {
                        let params = message.get("params");
                        let name = params.and_then(|p| p.get("name")).and_then(Value::as_str).unwrap_or_default();
                        let arguments = params.and_then(|p| p.get("arguments")).and_then(Value::as_object);
                        let config = crate::transcript::settings::config_dir();
                        let answer = match super::skill::prompt_get(&config, name, arguments) {
                            Ok(prompt) => reply(id, serde_json::to_value(prompt).unwrap_or_default()),
                            Err(error) => json!({ "jsonrpc": "2.0", "id": id,
                                "error": { "code": -32602, "message": error } }),
                        };
                        write_line(&mut output, &answer).await?;
                    }
                    (_, Some(id)) => {
                        let mut failure = NOT_RUNNING.to_string();
                        if app.is_none() {
                            let connected = match super::ipc::connect(&endpoint).await {
                                Ok(stream) => connect(stream, &hello).await,
                                Err(error) => Err(error),
                            };
                            match connected {
                                Ok(connected) => app = Some(connected),
                                Err(error) => failure = error,
                            }
                        }
                        let Some(connected) = &mut app else {
                            let answer = if method == "tools/call" {
                                tool_error(id, &failure)
                            } else {
                                json!({ "jsonrpc": "2.0", "id": id,
                                    "error": { "code": -32000, "message": failure } })
                            };
                            write_line(&mut output, &answer).await?;
                            continue;
                        };
                        if write_line(&mut connected.write, &message).await.is_ok() {
                            connected.pending.insert(id.to_string());
                        } else {
                            app = None;
                            write_line(&mut output, &tool_error(id, NOT_RUNNING)).await?;
                        }
                    }
                    // A notification (a cancel, say): only the app can use it.
                    (_, None) => {
                        if let Some(connected) = &mut app {
                            if write_line(&mut connected.write, &message).await.is_err() {
                                app = None;
                            }
                        }
                    }
                }
            }
            line = async {
                match &mut app {
                    Some(App { lines, .. }) => lines.next_line().await,
                    None => std::future::pending().await,
                }
            } => {
                match line {
                    Ok(Some(line)) => {
                        let Ok(message) = serde_json::from_str::<Value>(&line) else {
                            continue;
                        };
                        if let (Some(connected), Some(id)) = (&mut app, message.get("id")) {
                            connected.pending.remove(&id.to_string());
                        }
                        write_line(&mut output, &message).await?;
                    }
                    // The app closed (or was restarted): what was waiting hears so, and the next
                    // call connects again.
                    _ => {
                        if let Some(closed) = app.take() {
                            for id in closed.pending {
                                let id: Value = serde_json::from_str(&id).unwrap_or(Value::Null);
                                write_line(&mut output, &tool_error(&id, CLOSED_DURING_CALL)).await?;
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Copies the agent's messages to the app and the app's to the agent until either side
/// closes. (Tests talk to the app through it directly.)
pub async fn pump(
    stream: impl super::ipc::Stream,
    mut input: impl AsyncRead + Unpin,
    mut output: impl AsyncWrite + Unpin,
) -> std::io::Result<()> {
    let (mut from_app, mut to_app) = tokio::io::split(stream);
    let upstream = async {
        let mut buffer = vec![0u8; 64 * 1024];
        loop {
            let n = input.read(&mut buffer).await?;
            if n == 0 {
                return Ok::<_, std::io::Error>(());
            }
            to_app.write_all(&buffer[..n]).await?;
            to_app.flush().await?;
        }
    };
    let downstream = async {
        let mut buffer = vec![0u8; 64 * 1024];
        loop {
            let n = from_app.read(&mut buffer).await?;
            if n == 0 {
                return Ok::<_, std::io::Error>(());
            }
            output.write_all(&buffer[..n]).await?;
            output.flush().await?;
        }
    };
    tokio::select! {
        done = upstream => done,
        done = downstream => done,
    }
}
