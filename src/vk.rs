//! Win32 virtual-key codes and hotkey modifier constants.
//!
//! Centralising these avoids magic hex numbers scattered through
//! `overlay.rs`, `hotkey.rs`, and any future input handling code.

// ── Hotkey modifiers (RegisterHotKey) ───────────────────────────────

pub const MOD_ALT: u32 = 0x0001;
pub const MOD_CONTROL: u32 = 0x0002;
pub const MOD_SHIFT: u32 = 0x0004;
pub const MOD_WIN: u32 = 0x0008;

// ── Alphanumeric virtual-key codes ────────────────────────────────

pub const VK_0: u32 = 0x30;
pub const VK_9: u32 = 0x39;
pub const VK_A: u32 = 0x41;
pub const VK_Z: u32 = 0x5A;

// ── Special / navigation keys ─────────────────────────────────────

pub const VK_ESCAPE: u32 = 0x1B;
pub const VK_SPACE: u32 = 0x20;
pub const VK_TAB: u32 = 0x09;
pub const VK_RETURN: u32 = 0x0D;
pub const VK_UP: u32 = 0x26;
pub const VK_DOWN: u32 = 0x28;
pub const VK_L: u32 = 0x4C;

// ── Function keys ───────────────────────────────────────────────

pub const VK_F1: u32 = 0x70;
pub const VK_F2: u32 = 0x71;
pub const VK_F3: u32 = 0x72;
pub const VK_F4: u32 = 0x73;
pub const VK_F5: u32 = 0x74;
pub const VK_F6: u32 = 0x75;
pub const VK_F7: u32 = 0x76;
pub const VK_F8: u32 = 0x77;
pub const VK_F9: u32 = 0x78;
pub const VK_F10: u32 = 0x79;
pub const VK_F11: u32 = 0x7A;
pub const VK_F12: u32 = 0x7B;
