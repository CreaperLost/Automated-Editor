//! One-click connect: adds AeroEdits to an agent app's own MCP settings (or takes it out),
//! as `<this program> --mcp-stdio`. The file is backed up first and nothing else in it is
//! touched. Only the user's click in the AI agents dialog calls this.
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const NAME: &str = "aeroedits";
const ARG: &str = "--mcp-stdio";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum App {
    /// Chats in the Claude desktop app (`claude_desktop_config.json`).
    ClaudeApp,
    /// The Codex app and CLI (`~/.codex/config.toml`, which both read).
    Codex,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    pub app: App,
    /// The app's settings folder exists.
    pub installed: bool,
    /// Its settings start AeroEdits (this copy of it, or another).
    pub connected: bool,
    /// They start a different copy of AeroEdits than this one.
    pub other_copy: bool,
    pub config_path: String,
}

fn config_path(app: App) -> Option<PathBuf> {
    match app {
        App::ClaudeApp => claude_app_config(),
        App::Codex => Some(match std::env::var_os("CODEX_HOME") {
            Some(home) if !home.is_empty() => PathBuf::from(home).join("config.toml"),
            _ => dirs::home_dir()?.join(".codex").join("config.toml"),
        }),
    }
}

/// The Claude app's settings file. The Windows Store (MSIX) build keeps its `%APPDATA%\Claude`
/// in `%LOCALAPPDATA%\Packages\Claude_*\LocalCache\Roaming\Claude`, which only processes inside
/// its package see at the usual place; a copy started from the Start menu has to look there.
fn claude_app_config() -> Option<PathBuf> {
    let classic = dirs::config_dir()?.join("Claude");
    let mut folders = packaged_claude_folders();
    folders.push(classic.clone());
    let folder = pick_folder(&folders).unwrap_or(classic);
    Some(folder.join("claude_desktop_config.json"))
}

fn packaged_claude_folders() -> Vec<PathBuf> {
    if !cfg!(windows) {
        return Vec::new();
    }
    let Some(packages) = dirs::data_local_dir().map(|d| d.join("Packages")) else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(&packages) else {
        return Vec::new();
    };
    let mut folders: Vec<PathBuf> = entries
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("Claude_"))
        .map(|e| e.path().join("LocalCache").join("Roaming").join("Claude"))
        .collect();
    folders.sort();
    folders
}

/// The first folder that already has the settings file, else the first that exists.
fn pick_folder(folders: &[PathBuf]) -> Option<PathBuf> {
    folders
        .iter()
        .find(|f| f.join("claude_desktop_config.json").is_file())
        .or_else(|| folders.iter().find(|f| f.is_dir()))
        .cloned()
}

pub fn states(exe: &Path) -> Vec<AppState> {
    [App::ClaudeApp, App::Codex]
        .into_iter()
        .filter_map(|app| {
            let path = config_path(app)?;
            Some(state_at(app, &path, exe))
        })
        .collect()
}

fn state_at(app: App, path: &Path, exe: &Path) -> AppState {
    let command = fs::read_to_string(path)
        .ok()
        .and_then(|text| configured_command(app, &text));
    AppState {
        app,
        installed: path.parent().is_some_and(Path::is_dir),
        connected: command.is_some(),
        other_copy: command.is_some_and(|c| Path::new(&c) != exe),
        config_path: path.to_string_lossy().into_owned(),
    }
}

/// The program the settings start for AeroEdits, if they have an entry.
fn configured_command(app: App, text: &str) -> Option<String> {
    match app {
        App::ClaudeApp => {
            let value: serde_json::Value = serde_json::from_str(text).ok()?;
            Some(value["mcpServers"][NAME]["command"].as_str()?.to_string())
        }
        App::Codex => {
            let doc: toml_edit::DocumentMut = text.parse().ok()?;
            Some(doc.get("mcp_servers")?.get(NAME)?.get("command")?.as_str()?.to_string())
        }
    }
}

pub fn connect(app: App, exe: &Path) -> Result<AppState, String> {
    let path = config_path(app).ok_or("Could not find that app's settings folder")?;
    change(app, &path, exe, true)
}

pub fn disconnect(app: App, exe: &Path) -> Result<AppState, String> {
    let path = config_path(app).ok_or("Could not find that app's settings folder")?;
    change(app, &path, exe, false)
}

fn change(app: App, path: &Path, exe: &Path, add: bool) -> Result<AppState, String> {
    let dir = path.parent().ok_or("Bad settings path")?;
    if !dir.is_dir() {
        return Err(match app {
            App::ClaudeApp => "The Claude app does not seem to be installed".into(),
            App::Codex => "Codex does not seem to be installed".into(),
        });
    }
    let before = match fs::read_to_string(path) {
        Ok(text) => Some(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(format!("Could not read {}: {e}", path.display())),
    };
    let text = before.as_deref().unwrap_or_default();
    let after = match app {
        App::ClaudeApp => edit_json(text, exe, add)?,
        App::Codex => edit_toml(text, exe, add)?,
    };
    if before.as_deref() != Some(after.as_str()) {
        if let Some(before) = &before {
            let backup = path.with_extension(format!(
                "{}.aeroedits-backup",
                path.extension().and_then(|e| e.to_str()).unwrap_or("bak")
            ));
            fs::write(&backup, before).map_err(|e| format!("Could not back up the settings: {e}"))?;
        }
        fs::write(path, &after).map_err(|e| format!("Could not save {}: {e}", path.display()))?;
    }
    Ok(state_at(app, path, exe))
}

fn edit_json(text: &str, exe: &Path, add: bool) -> Result<String, String> {
    let mut value: serde_json::Value = if text.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(text).map_err(|e| {
            format!("The Claude app's settings are not valid JSON ({e}); fix them first")
        })?
    };
    let root = value
        .as_object_mut()
        .ok_or("The Claude app's settings are not a JSON object")?;
    let servers = root
        .entry("mcpServers")
        .or_insert_with(|| serde_json::json!({}));
    let servers = servers
        .as_object_mut()
        .ok_or("mcpServers in the Claude app's settings is not an object")?;
    if add {
        servers.insert(
            NAME.into(),
            serde_json::json!({ "command": exe.to_string_lossy(), "args": [ARG] }),
        );
    } else {
        servers.remove(NAME);
    }
    serde_json::to_string_pretty(&value)
        .map(|s| s + "\n")
        .map_err(|e| e.to_string())
}

fn edit_toml(text: &str, exe: &Path, add: bool) -> Result<String, String> {
    use toml_edit::{value, Array, DocumentMut, Item, Table};
    let mut doc: DocumentMut = text
        .parse()
        .map_err(|e| format!("Codex's config.toml does not parse ({e}); fix it first"))?;
    if add {
        let servers = doc
            .entry("mcp_servers")
            .or_insert_with(|| {
                let mut table = Table::new();
                table.set_implicit(true);
                Item::Table(table)
            })
            .as_table_mut()
            .ok_or("mcp_servers in Codex's config.toml is not a table")?;
        let mut entry = Table::new();
        entry["command"] = value(exe.to_string_lossy().as_ref());
        let mut args = Array::new();
        args.push(ARG);
        entry["args"] = value(args);
        servers.insert(NAME, Item::Table(entry));
    } else if let Some(servers) = doc.get_mut("mcp_servers").and_then(Item::as_table_mut) {
        servers.remove(NAME);
    }
    Ok(doc.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_settings_gain_and_lose_only_our_entry() {
        let exe = Path::new(r"C:\Program Files\AeroEdits\aeroedits.exe");
        let start = r#"{"mcpServers":{"other":{"command":"x"}},"theme":"dark"}"#;
        let added = edit_json(start, exe, true).unwrap();
        let value: serde_json::Value = serde_json::from_str(&added).unwrap();
        assert_eq!(value["theme"], "dark");
        assert_eq!(value["mcpServers"]["other"]["command"], "x");
        assert_eq!(value["mcpServers"]["aeroedits"]["args"][0], "--mcp-stdio");
        assert_eq!(
            configured_command(App::ClaudeApp, &added).as_deref(),
            Some(r"C:\Program Files\AeroEdits\aeroedits.exe")
        );
        let removed = edit_json(&added, exe, false).unwrap();
        assert_eq!(configured_command(App::ClaudeApp, &removed), None);
        assert!(removed.contains("\"other\""));
        assert!(edit_json("{ not json", exe, true).is_err());
        assert!(edit_json("", exe, true).unwrap().contains("aeroedits"));
    }

    #[test]
    fn codex_config_keeps_its_comments_and_other_servers() {
        let exe = Path::new(r"C:\Program Files\AeroEdits\aeroedits.exe");
        let start = "# my settings\nmodel = \"gpt-5\"\n\n[mcp_servers.docs]\nurl = \"https://x\"\n";
        let added = edit_toml(start, exe, true).unwrap();
        assert!(added.starts_with("# my settings\nmodel = \"gpt-5\"\n"), "{added}");
        assert!(added.contains("[mcp_servers.docs]"), "{added}");
        assert!(added.contains("[mcp_servers.aeroedits]"), "{added}");
        assert_eq!(
            configured_command(App::Codex, &added).as_deref(),
            Some(r"C:\Program Files\AeroEdits\aeroedits.exe")
        );
        let doc: toml_edit::DocumentMut = added.parse().unwrap();
        assert_eq!(doc["mcp_servers"]["aeroedits"]["args"][0].as_str(), Some("--mcp-stdio"));
        let removed = edit_toml(&added, exe, false).unwrap();
        assert_eq!(configured_command(App::Codex, &removed), None);
        assert!(removed.contains("[mcp_servers.docs]"));
        assert!(edit_toml("not = = toml", exe, true).is_err());
    }

    #[test]
    fn claude_settings_folder_prefers_one_with_the_file_then_one_that_exists() {
        let dir = tempfile::tempdir().unwrap();
        let packaged = dir.path().join("packaged");
        let classic = dir.path().join("classic");
        let missing = dir.path().join("missing");
        fs::create_dir(&packaged).unwrap();
        fs::create_dir(&classic).unwrap();
        let folders = [missing.clone(), packaged.clone(), classic.clone()];
        assert_eq!(pick_folder(&folders), Some(packaged.clone()));
        fs::write(classic.join("claude_desktop_config.json"), "{}").unwrap();
        assert_eq!(pick_folder(&folders), Some(classic));
        assert_eq!(pick_folder(&[missing]), None);
    }

    #[test]
    fn connecting_backs_up_the_old_file_and_reports_the_state() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "model = \"x\"\n").unwrap();
        let exe = dir.path().join("aeroedits.exe");
        let state = change(App::Codex, &path, &exe, true).unwrap();
        assert!(state.installed && state.connected && !state.other_copy);
        assert_eq!(
            fs::read_to_string(dir.path().join("config.toml.aeroedits-backup")).unwrap(),
            "model = \"x\"\n"
        );
        let elsewhere = state_at(App::Codex, &path, &dir.path().join("other.exe"));
        assert!(elsewhere.other_copy);
        let state = change(App::Codex, &path, &exe, false).unwrap();
        assert!(!state.connected);
    }
}
