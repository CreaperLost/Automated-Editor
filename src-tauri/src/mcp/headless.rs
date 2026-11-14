//! `aeroedits serve`: the app's MCP server with no window, for cloud sessions, CI and scripts.
//! It listens on the same private socket or pipe as the app ([`super::ipc`]), so
//! `aeroedits do` and `--mcp-stdio` reach it as they would reach the running editor. It runs
//! until stopped (Ctrl-C). Every tool works as in the app; nothing redraws.
use super::Host;
use crate::commands::{transcript::TranscriptState, AppState};
use std::sync::Arc;

const USAGE: &str = "Usage: aeroedits serve";

struct Headless {
    state: AppState,
    transcripts: TranscriptState,
}

impl Host for Headless {
    fn state(&self) -> &AppState {
        &self.state
    }

    fn transcripts(&self) -> &TranscriptState {
        &self.transcripts
    }

    /// No local model is downloaded in a headless run.
    fn local_review_ready(&self) -> bool {
        false
    }
}

pub fn run(args: &[String]) -> i32 {
    if !args.is_empty() {
        eprintln!("{USAGE}");
        return 2;
    }
    let host: Arc<dyn Host> = Arc::new(Headless {
        state: AppState::new(),
        transcripts: TranscriptState::default(),
    });
    let endpoint = match super::ipc::default_endpoint() {
        Ok(endpoint) => endpoint,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };
    if let Err(error) = super::start(host, endpoint.clone()) {
        eprintln!("{error}");
        return 1;
    }
    eprintln!(
        "AeroEdits is serving agents on {} (Ctrl-C stops it)",
        endpoint.0
    );
    loop {
        std::thread::park();
    }
}
