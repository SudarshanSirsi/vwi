# VWi (Window Wizard)

A lightweight, keyboard-driven window switcher for Windows. Switch between VS Code projects, browser windows, terminals, or any application instantly — without Alt+Tab, mouse, or taskbar hunting.

## Why VWi?

When you work with multiple VS Code windows across different repositories, switching becomes painful:
- **Alt+Tab** cycles through *every* window
- **Mouse** breaks keyboard flow
- **Taskbar** requires precision clicking

VWi gives each project a single-key shortcut. Press your global hotkey, see the list, press a key — done.

## Features

- **Global hotkey** — configurable, works from anywhere
- **Single-key switching** — no Enter needed; press a key and switch instantly
- **TOML config** — add projects without recompiling
- **Runtime reload** — edit config and press F5 in the overlay
- **Customizable UI** — colors, sizes, padding, and max height via config
- **Scrollable overlay** — cap the overlay height with `max_height`; scroll via mouse wheel or arrow keys
- **Diagnostic list mode** — press `L` to see all visible windows with titles (useful for writing `match` patterns)
- **Multi-monitor aware** — overlay appears centered on the monitor of the currently focused window
- **System tray icon** — orange VWi icon in the system tray with right-click menu (Show Overlay, Reload Config, Open Config Folder, Quit)
- **File logging** — writes to `%APPDATA%\vwi\vwi.log` for debugging
- **Embedded icon** — icon ships inside `vwi.exe`; no separate `.ico` file needed at runtime
- **Preserves window state** — maximized windows stay maximized
- **Single EXE** — no installer, no dependencies

## Quick Start

### 1. Build

**Windows (PowerShell):**

```powershell
# Requires Rust (https://rustup.rs/)
.\build.ps1

# Optional: specify a custom output path
.\build.ps1 -OutputPath "C:\Tools\vwi.exe"
```

**Linux / macOS / Git Bash / WSL:**

```bash
# Make executable and run
chmod +x build.sh
./build.sh

# Optional: specify a custom output path
./build.sh /usr/local/bin/vwi
```

**Or manually:**

```bash
cargo build --release
# Windows binary: target\release\vwi.exe
# Unix binary:    target/release/vwi
```

### 2. Create Config

```powershell
mkdir "$env:APPDATA\vwi"
Copy-Item config.example.toml "$env:APPDATA\vwi\config.toml"
notepad "$env:APPDATA\vwi\config.toml"
```

### 3. Run

```powershell
.\vwi.exe
```

Press your configured hotkey (default: `Ctrl+Shift+Space`), then press a project key to switch.

The app runs silently in the background with a system tray icon. Right-click the icon for quick actions.

## System Tray

When VWi is running, an orange icon appears in your system tray. Right-click it for:

| Menu Item | Action |
|-----------|--------|
| **Show Overlay** | Opens the switcher overlay immediately |
| **Reload Config** | Reloads `config.toml` without restarting |
| **Open Config Folder** | Opens `%APPDATA%\vwi` in File Explorer |
| **Quit** | Exits VWi completely |

## Logging

VWi writes a log file to `%APPDATA%\vwi\vwi.log` (e.g. `C:\Users\<name>\AppData\Roaming\vwi\vwi.log`).

This is useful for:
- Verifying which windows are detected
- Checking config load success or parse errors
- Diagnosing hotkey registration failures

Logs are also mirrored to stdout/stderr, so you still see output when running from a terminal.

## Configuration

Config lives at `%APPDATA%\vwi\config.toml`.

### Example

```toml
hotkey = "Ctrl+Alt+W"

[projects.backend]
key = "b"
match = "credilinq.backend"

[projects.react]
key = "r"
match = "alexi.react"

[projects.poc]
key = "p"
match = "alexi.poc.keystone"

[projects.alexi]
key = "a"
match = "credilinq.alexi"
```

### Hotkey Format

Supported modifiers: `Alt`, `Ctrl`, `Shift`, `Win`  
Supported keys: `A-Z`, `0-9`, `Space`, `Tab`, `Enter`, `Esc`, `F1`-`F12`

Examples:
- `Alt+Space`
- `Ctrl+Shift+W`
- `Win+Q`
- `Ctrl+Alt+F1`

> **Note:** `Alt+Space` is reserved by Windows for the active window's system menu. It may not work reliably as a global hotkey.

### UI Customization (Optional)

```toml
[ui]
show_overlay = true         # set to false for silent mode (no popup)
max_height = 400            # cap overlay height; scrollbar if content exceeds (0 = auto-fit)
font_height = 18
line_height = 32
pad_x = 24
pad_y = 16
min_width = 560
key_color = 0xFFA500      # orange accent
text_color = 0xE0E0E0     # light gray
bg_color = 0x1A1A1A       # dark background
border_color = 0x333333   # subtle border
```

All fields have defaults. Uncomment only what you want to change.

#### Silent Mode

Set `show_overlay = false` to hide the popup entirely. The app still captures
your next keystroke and switches to the matching project, but nothing appears
on screen. Ideal once you've memorized all your key mappings.

### Matching Strategy

The `match` string is checked against the full window title. A window matches if:

```rust
title.contains(match_pattern)
```

This means VS Code windows like `credilinq.backend - Devin - shareholders.service.ts` match `credilinq.backend` regardless of the current file.

## Overlay Controls

| Key | Action |
|-----|--------|
| `a-z`, `0-9` | Switch to the matching project |
| `Esc` | Dismiss overlay without switching |
| `F5` | Reload config from disk |
| `L` | Toggle diagnostic list (shows all visible windows with titles) |
| `↑` / `↓` | Scroll the overlay when `max_height` is set |
| Mouse wheel | Scroll the overlay when `max_height` is set |
| Click outside | Dismiss overlay |

## Auto-Start on Login

To run VWi automatically when Windows starts:

1. Press `Win + R`, type `shell:startup`, press Enter
2. Create a shortcut to `vwi.exe` in that folder

## Architecture

| File | Responsibility |
|------|---------------|
| `main.rs` | Entry point |
| `app.rs` | Message loop orchestration |
| `config.rs` | TOML config loading from `%APPDATA%` |
| `models.rs` | Shared structs: `Config`, `WindowInfo`, `UiConfig` |
| `windows.rs` | Win32 `EnumWindows`, `SetForegroundWindow` |
| `switcher.rs` | Window matching logic and config reload |
| `hotkey.rs` | Global hotkey registration via `RegisterHotKey` |
| `overlay.rs` | Borderless popup, GDI rendering, keyboard input |

## Troubleshooting

### Overlay doesn't appear
- Check console output for "Registering global hotkey" — if it says the registration failed, the hotkey may be used by another app
- Try a different hotkey like `Ctrl+Alt+W`

### Config changes don't take effect
- Press F5 inside the overlay to reload
- Or restart the app
- Check console for parse errors

### Window switching doesn't work
- Ensure the window title contains your `match` string
- Run with `cargo run` to see debug output showing matched windows
- Check `%APPDATA%\vwi\vwi.log` for detailed logs

### Overlay appears on the wrong monitor
- The overlay is shown on the monitor of the currently focused window
- If no window has focus, it falls back to the primary monitor

## License

MIT
