use crate::logger;
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, WPARAM},
        UI::Input::KeyboardAndMouse::{
            RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS,
        },
    },
};

/// Arbitrary ID we assign to our global hotkey.  Used both when registering
/// and when checking WM_HOTKEY messages later.
pub const HOTKEY_ID: i32 = 1;

/// Parses a human-readable hotkey string (e.g. "Ctrl+Alt+W") and registers
/// it as a system-wide hotkey via RegisterHotKey.
///
/// Why we need a custom parser:
///   The Win32 API expects separate modifier flags and a virtual-key code.
///   TOML lets the user write "Ctrl+Alt+W", so we split on '+' and map
///   each token to the corresponding MOD_* constant or VK_* value.
///
/// Supported tokens:
///   Modifiers: Alt, Ctrl/Control, Shift, Win/Windows/Super
///   Special keys: Space, Tab, Enter/Return, Esc/Escape, F1-F12
///   Any single letter or digit is treated as its VK code.
pub fn parse_and_register(hwnd: HWND, hotkey_str: &str) {
    let parts: Vec<&str> = hotkey_str.split('+').map(|s| s.trim()).collect();
    let mut modifiers = HOT_KEY_MODIFIERS(0);
    let mut key_vk: u32 = 0;

    for part in &parts {
        match *part {
            "Alt" => modifiers = HOT_KEY_MODIFIERS(modifiers.0 | 0x0001),            // MOD_ALT
            "Ctrl" | "Control" => modifiers = HOT_KEY_MODIFIERS(modifiers.0 | 0x0002), // MOD_CONTROL
            "Shift" => modifiers = HOT_KEY_MODIFIERS(modifiers.0 | 0x0004),          // MOD_SHIFT
            "Win" | "Windows" | "Super" => modifiers = HOT_KEY_MODIFIERS(modifiers.0 | 0x0008), // MOD_WIN
            k if k.len() == 1 => {
                let c = k.chars().next().unwrap();
                key_vk = c.to_ascii_uppercase() as u32;
            }
            "Space" => key_vk = 0x20,   // VK_SPACE
            "Tab" => key_vk = 0x09,     // VK_TAB
            "Enter" | "Return" => key_vk = 0x0D, // VK_RETURN
            "Esc" | "Escape" => key_vk = 0x1B,   // VK_ESCAPE
            "F1" => key_vk = 0x70, "F2" => key_vk = 0x71, "F3" => key_vk = 0x72,
            "F4" => key_vk = 0x73, "F5" => key_vk = 0x74, "F6" => key_vk = 0x75,
            "F7" => key_vk = 0x76, "F8" => key_vk = 0x77, "F9" => key_vk = 0x78,
            "F10" => key_vk = 0x79, "F11" => key_vk = 0x7A, "F12" => key_vk = 0x7B,
            _ => logger::error(&format!("Warning: unknown hotkey part '{}'", part)),
        }
    }

    // If parsing failed entirely, fall back to a safe default so the app
    // doesn't silently fail to register any hotkey.
    if key_vk == 0 {
        logger::error(&format!("Warning: could not parse hotkey '{}', defaulting to 'Ctrl+Shift+W'", hotkey_str));
        modifiers = HOT_KEY_MODIFIERS(0x0002 | 0x0004); // Ctrl+Shift
        key_vk = 'W' as u32;
    }

    logger::info(&format!(
        "Registering global hotkey: {} (vk=0x{:02X}, mods=0x{:04X})",
        hotkey_str, key_vk, modifiers.0
    ));

    unsafe {
        let result = RegisterHotKey(Some(hwnd), HOTKEY_ID, modifiers, key_vk);
        if result.is_err() {
            logger::error(&format!("Failed to register hotkey: {:?}", result));
        }
    }
}

/// Clean up the hotkey registration on shutdown so Windows doesn't keep
/// the shortcut bound to a dead process.
pub fn unregister(hwnd: HWND) {
    unsafe {
        let _ = UnregisterHotKey(Some(hwnd), HOTKEY_ID);
    }
}

/// Checks whether a WM_HOTKEY message was triggered by *our* registered hotkey.
/// We only register one hotkey, so we simply compare wParam to HOTKEY_ID.
pub fn is_hotkey_msg(wparam: WPARAM, _lparam: LPARAM) -> bool {
    wparam.0 == HOTKEY_ID as usize
}
