# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-06-13

### Added

- Global configurable hotkey to open the overlay (`Ctrl+Shift+Space` by default)
- Single-key project switching — no Enter key required
- TOML-based configuration with runtime reload (press F5 in overlay)
- Project matching by window title substring
- Borderless overlay with customizable UI (colors, fonts, padding, sizes)
- Overlay height capping via `max_height` config with scrollbar
- Mouse wheel and arrow key scrolling for capped overlay
- Diagnostic list mode (press `L`) showing all visible windows with titles
- Multi-monitor support — overlay appears on the active window's monitor
- System tray icon with right-click menu (Show Overlay, Reload Config, Open Config Folder, Quit)
- File logging to `%APPDATA%\vwi\vwi.log`
- Preserves window state on switch (maximized windows stay maximized)
- Single portable EXE — no installer or dependencies
