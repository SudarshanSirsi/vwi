use crate::defaults::*;
use std::collections::HashMap;
use windows::Win32::Foundation::HWND;

/// Represents a single visible top-level window discovered by EnumWindows.
/// hwnd is the Win32 window handle; title is the full window caption text.
#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub hwnd: HWND,
    pub title: String,
}

/// Per-project configuration loaded from the TOML file.
/// `key` is the single character the user presses to switch to this project.
/// `match` is the substring we search for inside window titles.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ProjectConfig {
    pub key: String,
    pub r#match: String,
}

/// Top-level configuration struct, deserialized from %APPDATA%\vwi\config.toml.
/// `hotkey` defines the global shortcut that opens the overlay (e.g. "Ctrl+Alt+W").
/// `projects` is a map of arbitrary names to ProjectConfig entries.
/// Optional UI customization.  All fields have defaults so the user
/// doesn't need to specify them; they only need to override what they want.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_font_height")]
    pub font_height: i32,
    #[serde(default = "default_line_height")]
    pub line_height: i32,
    #[serde(default = "default_pad_x")]
    pub pad_x: i32,
    #[serde(default = "default_pad_y")]
    pub pad_y: i32,
    #[serde(default = "default_min_width")]
    pub min_width: i32,
    #[serde(default = "default_key_color")]
    pub key_color: u32,
    #[serde(default = "default_text_color")]
    pub text_color: u32,
    #[serde(default = "default_bg_color")]
    pub bg_color: u32,
    #[serde(default = "default_border_color")]
    pub border_color: u32,
    #[serde(default = "default_show_overlay")]
    pub show_overlay: bool,
    /// Maximum overlay height in pixels. 0 means unlimited (auto-fit).
    /// When content exceeds this height, a scrollbar appears and the
    /// window is capped at this size.
    #[serde(default = "default_max_height")]
    pub max_height: i32,
    /// Size of window icons drawn next to each item, in pixels.
    /// 0 disables icons.
    #[serde(default = "default_icon_size")]
    pub icon_size: i32,
    /// Background color of the keyboard-key badge behind shortcut keys.
    #[serde(default = "default_key_box_color")]
    pub key_box_color: u32,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            font_height: FONT_HEIGHT,
            line_height: LINE_HEIGHT,
            pad_x: PAD_X,
            pad_y: PAD_Y,
            min_width: MIN_WIDTH,
            key_color: KEY_COLOR,
            text_color: TEXT_COLOR,
            bg_color: BG_COLOR,
            border_color: BORDER_COLOR,
            show_overlay: SHOW_OVERLAY,
            max_height: MAX_HEIGHT,
            icon_size: ICON_SIZE,
            key_box_color: KEY_BOX_COLOR,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Config {
    #[serde(default = "default_hotkey")]
    pub hotkey: String,
    #[serde(default)]
    pub projects: HashMap<String, ProjectConfig>,
    #[serde(default)]
    pub ui: UiConfig,
}

impl Default for Config {
    /// Used when no config file exists yet.
    /// Default hotkey avoids "Alt+Space" because Windows reserves it for
    /// the active window's system menu, which would prevent our global hook
    /// from ever firing.
    fn default() -> Self {
        Self {
            hotkey: HOTKEY.to_string(),
            projects: HashMap::new(),
            ui: UiConfig::default(),
        }
    }
}
