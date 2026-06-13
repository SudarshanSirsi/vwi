// VWi (Window Wizard) - Keyboard-Driven VS Code Window Switcher for Windows
//
// Entry point. All modules are declared here and the application
// is started by calling app::run().

#![windows_subsystem = "windows"]

mod app;      // Main application loop and orchestration
mod config;   // TOML configuration loading
mod hotkey;   // Global hotkey registration via Win32 RegisterHotKey
mod logger;   // File logging to %APPDATA%\vwi\vwi.log
mod models;   // Shared data structures (Config, WindowInfo, etc.)
mod overlay;  // Borderless popup window for the switcher UI
mod switcher; // Window matching logic and focus switching
mod tray;     // System tray icon with right-click menu
mod windows;  // Win32 window enumeration and activation helpers

fn main() {
    // Run the application. Any top-level error (e.g. config parse failure)
    // is printed to stderr and the process exits with code 1.
    if let Err(e) = app::run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}