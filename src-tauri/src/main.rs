// AeroEdits entry point
// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
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
