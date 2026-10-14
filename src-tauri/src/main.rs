// AeroEdits entry point
// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // An MCP client (the Claude app's chat, say) started this copy to reach the running
    // editor: bridge stdin and stdout to it, with no window of its own.
    if std::env::args().any(|arg| arg == "--mcp-stdio") {
        std::process::exit(aeroedits_lib::mcp::bridge::run());
    }
    // `aeroedits do <action> ...`: an agent's shell (or a script) drives the running editor.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("do") {
        std::process::exit(aeroedits_lib::mcp::cli::run(&args[1..]));
    }

    #[cfg(feature = "tauri-app")]
    {
        aeroedits_lib::run();
    }

    #[cfg(not(feature = "tauri-app"))]
    {
        println!("AeroEdits v0.1.0 (headless mode)");
        println!("  Editor core engine initialized without the Tauri GUI shell.");
        println!("  To build the desktop GUI, compile with `--features tauri-app`.");
    }
}
