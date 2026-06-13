//! Central place for all application defaults.
//!
//! Keeping every literal default in one file makes it trivial to audit,
//! change, or document the out-of-the-box behaviour without hunting through
//! struct definitions.

// ── UI ──────────────────────────────────────────────────────────────

pub const FONT_HEIGHT: i32 = 18;
pub const LINE_HEIGHT: i32 = 32;
pub const PAD_X: i32 = 24;
pub const PAD_Y: i32 = 16;
pub const MIN_WIDTH: i32 = 560;

// colours in 0xRRGGBB
pub const KEY_COLOR: u32 = 0xFFA500;      // orange accent
pub const TEXT_COLOR: u32 = 0xE0E0E0;     // light gray
pub const BG_COLOR: u32 = 0x1A1A1A;       // dark background
pub const BORDER_COLOR: u32 = 0x333333;   // subtle border
pub const KEY_BOX_COLOR: u32 = 0x333333;  // keyboard badge background

pub const SHOW_OVERLAY: bool = true;
pub const MAX_HEIGHT: i32 = 0;            // 0 = auto-fit
pub const ICON_SIZE: i32 = 14;            // 0 = hide icons

// ── layout spacing (pixels) ────────────────────────────────────────

/// Gap between the key badge and the window icon in normal mode.
pub const KEY_ICON_GAP: i32 = 22;
/// Gap between the index number and the window icon in list mode.
pub const LIST_NUM_ICON_GAP: i32 = 22;
/// Gap between the icon and the title text.
pub const ICON_TITLE_GAP: i32 = 8;

// ── behaviour ──────────────────────────────────────────────────────

pub const HOTKEY: &str = "Ctrl+Shift+Space";

// ── serde helpers (kept for #[serde(default = "...")] ) ────────────

pub fn default_font_height() -> i32 { FONT_HEIGHT }
pub fn default_line_height() -> i32 { LINE_HEIGHT }
pub fn default_pad_x() -> i32 { PAD_X }
pub fn default_pad_y() -> i32 { PAD_Y }
pub fn default_min_width() -> i32 { MIN_WIDTH }
pub fn default_key_color() -> u32 { KEY_COLOR }
pub fn default_text_color() -> u32 { TEXT_COLOR }
pub fn default_bg_color() -> u32 { BG_COLOR }
pub fn default_border_color() -> u32 { BORDER_COLOR }
pub fn default_show_overlay() -> bool { SHOW_OVERLAY }
pub fn default_max_height() -> i32 { MAX_HEIGHT }
pub fn default_icon_size() -> i32 { ICON_SIZE }
pub fn default_key_box_color() -> u32 { KEY_BOX_COLOR }
pub fn default_hotkey() -> String { HOTKEY.to_string() }
