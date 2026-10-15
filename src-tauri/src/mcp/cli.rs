//! `aeroedits do <action> [--key value ...]`: one of the app's tools from a shell. Agents with a
//! shell (Claude Code, Codex) drive the running app this way without an MCP tool list in their
//! context; people can script it too. It speaks MCP to the app over the same private pipe as
//! `--mcp-stdio` ([`super::ipc`]), prints the answer on stdout and progress on stderr.
//!
//! ```text
//! aeroedits do                          the actions and tools
//! aeroedits do silence --help           what one takes
//! aeroedits do silence --preset tight --min-duration-ms 150
//! aeroedits do settings captions --set '{"enabled": true}'
//! ```
use serde_json::{json, Map, Value};
use std::io::Write;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const USAGE: &str = "Usage: aeroedits do [<action> [--key value ...] | <action> --help]";

/// What was asked on the command line.
#[derive(Debug, PartialEq)]
pub enum Request {
    /// List the actions and tools.
    List,
    /// Explain one.
    Help(String),
    /// Run one with these arguments (`--key value` pairs, not yet typed).
    Call {
        tool: String,
        positional: Vec<String>,
        named: Vec<(String, Option<String>)>,
    },
}

/// `min-duration-ms` → `minDurationMs`.
fn camel(key: &str) -> String {
    let mut out = String::new();
    let mut upper = false;
    for c in key.chars() {
        if c == '-' || c == '_' {
            upper = !out.is_empty();
        } else if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

pub fn parse(args: &[String]) -> Result<Request, String> {
    let Some(first) = args.first() else {
        return Ok(Request::List);
    };
    if matches!(first.as_str(), "--help" | "-h" | "help" | "list" | "--list") {
        return Ok(match args.get(1) {
            Some(tool) if first == "help" => Request::Help(tool.replace('-', "_")),
            _ => Request::List,
        });
    }
    let tool = first.replace('-', "_");
    let mut positional = Vec::new();
    let mut named = Vec::new();
    let mut rest = args[1..].iter().peekable();
    while let Some(arg) = rest.next() {
        if arg == "--help" || arg == "-h" {
            return Ok(Request::Help(tool));
        }
        let Some(key) = arg.strip_prefix("--") else {
            positional.push(arg.clone());
            continue;
        };
        if let Some((key, value)) = key.split_once('=') {
            named.push((camel(key), Some(value.to_string())));
        } else if let Some(key) = key.strip_prefix("no-") {
            named.push((camel(key), Some("false".into())));
        } else {
            let value = match rest.peek() {
                Some(next) if !next.starts_with("--") => rest.next().cloned(),
                _ => None,
            };
            named.push((camel(key), value));
        }
    }
    Ok(Request::Call {
        tool,
        positional,
        named,
    })
}

/// A property's schema, with a `$ref` (or an optional's `anyOf`) looked up in `root`.
fn resolve<'a>(root: &'a Value, schema: &'a Value) -> &'a Value {
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        let name = reference.rsplit('/').next().unwrap_or_default();
        for defs in ["$defs", "definitions"] {
            if let Some(found) = root.get(defs).and_then(|d| d.get(name)) {
                return resolve(root, found);
            }
        }
    }
    if let Some(options) = schema.get("anyOf").and_then(Value::as_array) {
        if let Some(found) = options
            .iter()
            .find(|o| o.get("type").and_then(Value::as_str) != Some("null"))
        {
            return resolve(root, found);
        }
    }
    schema
}

/// The JSON type a property takes: `string`, `integer`, `number`, `boolean`, `array`,
/// `object`, or `None` when the schema does not say.
fn json_type(schema: &Value) -> Option<&str> {
    match schema.get("type")? {
        Value::String(kind) => Some(kind),
        Value::Array(kinds) => kinds
            .iter()
            .filter_map(Value::as_str)
            .find(|k| *k != "null"),
        _ => None,
    }
}

/// The values an enum property takes.
fn choices(schema: &Value) -> Vec<String> {
    if let Some(values) = schema.get("enum").and_then(Value::as_array) {
        return values
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect();
    }
    schema
        .get("oneOf")
        .and_then(Value::as_array)
        .map(|options| {
            options
                .iter()
                .filter_map(|o| {
                    o.get("const")
                        .or_else(|| o.get("enum").and_then(|e| e.get(0)))
                        .and_then(Value::as_str)
                        .map(str::to_string)
                })
                .collect()
        })
        .unwrap_or_default()
}

/// A command-line value as the JSON the property wants.
fn typed(root: &Value, property: Option<&Value>, raw: Option<&str>) -> Result<Value, String> {
    let schema = property.map(|p| resolve(root, p));
    let kind = schema.and_then(json_type);
    let Some(raw) = raw else {
        // A bare `--flag`.
        return Ok(Value::Bool(true));
    };
    match kind {
        Some("string") => Ok(Value::String(raw.to_string())),
        Some("array") if !raw.trim_start().starts_with('[') => {
            let items = schema
                .and_then(|s| s.get("items"))
                .map(|i| resolve(root, i));
            raw.split(',')
                .map(|item| typed(root, items, Some(item.trim())))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array)
        }
        Some("object") | Some("array") => serde_json::from_str(raw)
            .map_err(|e| format!("Expected JSON for this value ({e}): {raw}")),
        _ => {
            Ok(serde_json::from_str::<Value>(raw)
                .unwrap_or_else(|_| Value::String(raw.to_string())))
        }
    }
}

/// The tool call's arguments from the command line, typed by the tool's schema.
pub fn arguments(
    schema: &Value,
    positional: &[String],
    named: &[(String, Option<String>)],
) -> Result<Value, String> {
    let properties = schema.get("properties").cloned().unwrap_or_default();
    let mut out = Map::new();
    // Plain words fill the required properties in order: `aeroedits do settings captions`; or
    // the only property of a tool that requires none: `aeroedits do template tutorial`.
    let mut required: Vec<&str> = schema
        .get("required")
        .and_then(Value::as_array)
        .map(|r| r.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    if required.is_empty() {
        if let Some(Value::Object(map)) = schema.get("properties") {
            if map.len() == 1 {
                required.extend(map.keys().map(String::as_str));
            }
        }
    }
    if positional.len() > required.len() {
        return Err(format!(
            "Too many plain values: {}. Name them with --key value.",
            positional.join(" ")
        ));
    }
    for (key, raw) in required.iter().zip(positional) {
        out.insert(
            key.to_string(),
            typed(schema, properties.get(*key), Some(raw))?,
        );
    }
    for (key, raw) in named {
        if key == "json" {
            let Some(Value::Object(extra)) =
                raw.as_deref().and_then(|r| serde_json::from_str(r).ok())
            else {
                return Err("--json needs a JSON object".into());
            };
            out.extend(extra);
            continue;
        }
        if !properties.get(key).is_some() {
            let known: Vec<String> = properties
                .as_object()
                .map(|p| p.keys().cloned().collect())
                .unwrap_or_default();
            return Err(format!(
                "Unknown option --{key}. Options: {}",
                if known.is_empty() {
                    "none".into()
                } else {
                    known.join(", ")
                }
            ));
        }
        out.insert(
            key.clone(),
            typed(schema, properties.get(key), raw.as_deref())?,
        );
    }
    Ok(Value::Object(out))
}

/// One tool's options, for `--help`.
pub fn help(tool: &Value) -> String {
    let schema = tool.get("inputSchema").cloned().unwrap_or_default();
    let mut out = format!(
        "{} — {}\n",
        tool["name"].as_str().unwrap_or_default(),
        tool["description"].as_str().unwrap_or_default()
    );
    let required: Vec<&str> = schema
        .get("required")
        .and_then(Value::as_array)
        .map(|r| r.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
        for (key, property) in properties {
            let resolved = resolve(&schema, property);
            let values = choices(resolved);
            let kind = if values.is_empty() {
                json_type(resolved).unwrap_or("value").to_string()
            } else {
                values.join("|")
            };
            let description = property
                .get("description")
                .or_else(|| resolved.get("description"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .replace('\n', " ");
            let flag: String = key
                .chars()
                .flat_map(|c| {
                    if c.is_uppercase() {
                        vec!['-', c.to_ascii_lowercase()]
                    } else {
                        vec![c]
                    }
                })
                .collect();
            out.push_str(&format!(
                "  --{flag} <{kind}>{}  {description}\n",
                if required.contains(&key.as_str()) {
                    " (required)"
                } else {
                    ""
                }
            ));
        }
    }
    out
}

fn first_sentence(text: &str) -> &str {
    text.split_inclusive(". ").next().unwrap_or(text).trim()
}

/// Who is calling, for the Agent panel: `AEROEDITS_AGENT`, else a guess from the shell.
fn caller() -> String {
    if let Ok(name) = std::env::var("AEROEDITS_AGENT") {
        if !name.trim().is_empty() {
            return name;
        }
    }
    if std::env::var_os("CLAUDECODE").is_some() {
        return "claude-code".into();
    }
    if std::env::var_os("CODEX_SANDBOX").is_some() || std::env::var_os("CODEX_HOME").is_some() {
        return "codex".into();
    }
    "cli".into()
}

/// Runs `aeroedits do ...`; returns the exit code: 0 done, 1 the action failed, 2 it could not
/// be asked (usage, or the app not reachable).
pub fn run(args: &[String]) -> i32 {
    attach_console();
    let request = match parse(args) {
        Ok(request) => request,
        Err(error) => {
            eprintln!("{error}\n{USAGE}");
            return 2;
        }
    };
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("aeroedits do: {error}");
            return 2;
        }
    };
    runtime.block_on(async {
        let stream = match super::ipc::default_endpoint() {
            Ok(endpoint) => super::ipc::connect(&endpoint).await,
            Err(error) => Err(error),
        };
        match stream {
            Ok(stream) => {
                let (mut out, mut err) = (std::io::stdout(), std::io::stderr());
                session(stream, request, &caller(), &mut out, &mut err).await
            }
            Err(error) => {
                eprintln!("{error}");
                2
            }
        }
    })
}

/// One request over `stream`: the handshake, then the call, printing to `out` and `err`.
pub async fn session(
    stream: impl super::ipc::Stream,
    request: Request,
    caller: &str,
    out: &mut impl Write,
    err: &mut impl Write,
) -> i32 {
    let (read, mut write) = tokio::io::split(stream);
    let mut lines = BufReader::new(read).lines();
    let mut send = async |message: Value| -> Result<(), String> {
        write
            .write_all(format!("{message}\n").as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        write.flush().await.map_err(|e| e.to_string())
    };
    fn fail(err: &mut impl Write, message: String) -> i32 {
        let _ = writeln!(err, "{message}");
        2
    }
    // The reply to request `id`; progress notifications on the way are shown.
    macro_rules! reply {
        ($id:expr) => {{
            let mut found = None;
            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        let Ok(message) = serde_json::from_str::<Value>(&line) else {
                            continue;
                        };
                        if message.get("method").and_then(Value::as_str)
                            == Some("notifications/progress")
                        {
                            let params = &message["params"];
                            let percent = params["progress"].as_f64().unwrap_or(0.0) * 100.0;
                            let _ = writeln!(
                                err,
                                "[{percent:3.0}%] {}",
                                params["message"].as_str().unwrap_or_default()
                            );
                            continue;
                        }
                        if message.get("id") == Some(&json!($id)) {
                            found = Some(message);
                            break;
                        }
                    }
                    _ => break,
                }
            }
            match found {
                Some(message) => match message.get("error") {
                    Some(error) => Err(error["message"]
                        .as_str()
                        .unwrap_or("Request failed")
                        .to_string()),
                    None => Ok(message["result"].clone()),
                },
                None => Err("AeroEdits closed the connection".to_string()),
            }
        }};
    }

    let hello = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
        "protocolVersion": "2025-06-18", "capabilities": {},
        "clientInfo": {"name": caller, "version": env!("CARGO_PKG_VERSION")}}});
    if let Err(error) = send(hello).await {
        return fail(err, error);
    }
    if let Err(error) = reply!(1) {
        return fail(err, error);
    }
    if let Err(error) = send(json!({"jsonrpc": "2.0", "method": "notifications/initialized"})).await
    {
        return fail(err, error);
    }
    if let Err(error) = send(json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"})).await {
        return fail(err, error);
    }
    let tools = match reply!(2) {
        Ok(result) => result["tools"].as_array().cloned().unwrap_or_default(),
        Err(error) => return fail(err, error),
    };
    let find = |name: &str| tools.iter().find(|t| t["name"] == name).cloned();
    let (tool, positional, named) = match request {
        Request::List => {
            let _ = writeln!(
                out,
                "Actions and tools (aeroedits do <name> --help for options):"
            );
            for tool in &tools {
                let _ = writeln!(
                    out,
                    "  {:<18} {}",
                    tool["name"].as_str().unwrap_or_default(),
                    first_sentence(tool["description"].as_str().unwrap_or_default())
                );
            }
            return 0;
        }
        Request::Help(name) => {
            return match find(&name) {
                Some(tool) => {
                    let _ = write!(out, "{}", help(&tool));
                    0
                }
                None => fail(err, format!("No action named {name}. {USAGE}")),
            };
        }
        Request::Call {
            tool,
            positional,
            named,
        } => (tool, positional, named),
    };
    let Some(definition) = find(&tool) else {
        return fail(
            err,
            format!("No action named {tool}; `aeroedits do` lists them."),
        );
    };
    let schema = definition.get("inputSchema").cloned().unwrap_or_default();
    let arguments = match arguments(&schema, &positional, &named) {
        Ok(arguments) => arguments,
        Err(error) => return fail(err, format!("{error}\n{}", help(&definition))),
    };
    let call = json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {
        "name": tool, "arguments": arguments, "_meta": {"progressToken": 1}}});
    if let Err(error) = send(call).await {
        return fail(err, error);
    }
    let result = match reply!(3) {
        Ok(result) => result,
        Err(error) => return fail(err, error),
    };
    let mut images = 0;
    for block in result["content"].as_array().cloned().unwrap_or_default() {
        match block["type"].as_str() {
            Some("text") => {
                let _ = writeln!(out, "{}", block["text"].as_str().unwrap_or_default());
            }
            Some("image") => {
                use base64::Engine;
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(block["data"].as_str().unwrap_or_default())
                    .unwrap_or_default();
                images += 1;
                let path = std::env::temp_dir().join(format!(
                    "aeroedits-{tool}-{}-{images}.png",
                    std::process::id()
                ));
                match std::fs::write(&path, bytes) {
                    Ok(()) => {
                        let _ = writeln!(out, "Image: {}", path.display());
                    }
                    Err(error) => {
                        let _ = writeln!(err, "Could not save the image: {error}");
                    }
                }
            }
            _ => {}
        }
    }
    if result["isError"] == true {
        1
    } else {
        0
    }
}

/// The release build has no console of its own: run from a terminal, write to the terminal's.
fn attach_console() {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Console::{
            AttachConsole, GetStdHandle, ATTACH_PARENT_PROCESS, STD_OUTPUT_HANDLE,
        };
        // Output already goes somewhere (a pipe from an agent, a file): leave it.
        if GetStdHandle(STD_OUTPUT_HANDLE).is_null() {
            AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn command_lines_become_requests() {
        assert_eq!(parse(&[]).unwrap(), Request::List);
        assert_eq!(
            parse(&strings(&["help", "mouth-noises"])).unwrap(),
            Request::Help("mouth_noises".into())
        );
        assert_eq!(
            parse(&strings(&["silence", "--help"])).unwrap(),
            Request::Help("silence".into())
        );
        assert_eq!(
            parse(&strings(&[
                "silence",
                "--preset",
                "tight",
                "--min-duration-ms=150",
                "--no-apply",
                "--force"
            ]))
            .unwrap(),
            Request::Call {
                tool: "silence".into(),
                positional: vec![],
                named: vec![
                    ("preset".into(), Some("tight".into())),
                    ("minDurationMs".into(), Some("150".into())),
                    ("apply".into(), Some("false".into())),
                    ("force".into(), None),
                ],
            }
        );
    }

    #[test]
    fn values_follow_the_schema() {
        let schema =
            serde_json::to_value(schemars::schema_for!(super::super::actions::FillerArgs)).unwrap();
        let args = arguments(
            &schema,
            &[],
            &[
                ("kinds".into(), Some("filler,retake".into())),
                ("ai".into(), None),
                ("stream".into(), Some("123".into())),
                ("apply".into(), Some("false".into())),
            ],
        )
        .unwrap();
        assert_eq!(
            args,
            json!({"kinds": ["filler", "retake"], "ai": true, "stream": "123", "apply": false})
        );
        let error = arguments(&schema, &[], &[("kind".into(), Some("x".into()))]).unwrap_err();
        assert!(
            error.contains("Unknown option --kind") && error.contains("kinds"),
            "{error}"
        );
        let silence =
            serde_json::to_value(schemars::schema_for!(super::super::actions::SilenceArgs))
                .unwrap();
        assert_eq!(
            arguments(&silence, &[], &[("preset".into(), Some("tight".into()))]).unwrap(),
            json!({"preset": "tight"})
        );

        let settings =
            serde_json::to_value(schemars::schema_for!(super::super::actions::SettingsArgs))
                .unwrap();
        let args = arguments(
            &settings,
            &strings(&["captions"]),
            &[("set".into(), Some(r#"{"enabled": true}"#.into()))],
        )
        .unwrap();
        assert_eq!(args, json!({"area": "captions", "set": {"enabled": true}}));

        // A tool with one optional option takes it as a plain word: `do template tutorial`.
        let template =
            serde_json::to_value(schemars::schema_for!(super::super::skill::TemplateArgs)).unwrap();
        assert_eq!(
            arguments(&template, &strings(&["tutorial"]), &[]).unwrap(),
            json!({"name": "tutorial"})
        );
        assert!(
            arguments(&silence, &strings(&["tight"]), &[]).is_err(),
            "silence has several"
        );
    }

    #[test]
    fn help_lists_options_with_their_choices() {
        let tool = json!({
            "name": "silence",
            "description": "Jump cuts.",
            "inputSchema": serde_json::to_value(schemars::schema_for!(super::super::actions::SilenceArgs)).unwrap(),
        });
        let text = help(&tool);
        assert!(
            text.contains("--preset <gentle|balanced|tight|aggressive>"),
            "{text}"
        );
        assert!(text.contains("--min-duration-ms <integer>"), "{text}");
    }
}
