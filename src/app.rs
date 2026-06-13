use crate::config::load_config;
use crate::hotkey;
use crate::logger;
use crate::overlay::{create_overlay, show_overlay, show_overlay_silent};
use crate::switcher::Switcher;
use crate::tray;
use std::cell::RefCell;
use std::rc::Rc;
use windows::{
    Win32::{
        UI::WindowsAndMessaging::{
            GetMessageW, TranslateMessage, DispatchMessageW, WM_HOTKEY, MSG,
        },
    },
};

/// Main application orchestrator.  Sets up the config, overlay window,
/// global hotkey, and then enters the classic Windows message loop.
///
/// Architecture note:
///   We use Rc<RefCell<Switcher>> because the Switcher is accessed from
///   two places: the message loop (when hotkey fires) and the window
///   procedure (when a key is pressed inside the overlay).  Both need
///   mutable access at different times, so interior mutability via
///   RefCell is the simplest safe choice.
pub fn run() -> anyhow::Result<()> {
    // Start logging early so we can diagnose any startup failures.
    logger::init();

    let config = load_config()?;
    logger::info(&format!(
        "Config loaded: {} projects, hotkey={}",
        config.projects.len(),
        config.hotkey
    ));

    // Wrap the Switcher in Rc so both the message loop and the overlay
    // window proc can hold a reference to the same instance.
    let switcher = Rc::new(RefCell::new(Switcher::new(config)));

    // Create the borderless popup window.  It starts hidden.
    let hwnd = create_overlay(switcher.clone());

    // Add a system tray icon so the user knows VWi is running and can
    // right-click for a menu (Reload Config, Open Config Folder, Quit).
    tray::create_tray(hwnd);

    // Read the hotkey string from config and register it system-wide.
    let hotkey_str = switcher.borrow().get_config().hotkey.clone();
    hotkey::parse_and_register(hwnd, &hotkey_str);

    // Standard Win32 message pump.  GetMessageW blocks until a message
    // arrives, keeping CPU usage at zero while idle.
    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        loop {
            let ret = GetMessageW(&mut msg, None, 0, 0);
            // ret.0 == 0  -> WM_QUIT received, time to shut down.
            // ret.0 == -1 -> error (rare), bail out.
            if ret.0 == 0 {
                break;
            }
            if ret.0 == -1 {
                break;
            }

            if msg.message == WM_HOTKEY && hotkey::is_hotkey_msg(msg.wParam, msg.lParam) {
                // Hotkey fired: refresh the window list, compute matches,
                // print debug info, and show the overlay centered on screen.
                logger::info("Hotkey pressed!");
                switcher.borrow_mut().refresh();
                let st = switcher.borrow();
                let matches = st.matches();
                let show_ui = st.get_config().ui.show_overlay;
                logger::info(&format!("Found {} matches:", matches.len()));
                for (key, title, _hwnd) in &matches {
                    logger::info(&format!("  {} -> {}", key, title));
                }
                drop(st); // explicit drop to end the borrow before showing overlay
                if show_ui {
                    show_overlay(hwnd);
                } else {
                    // Silent mode: activate the first match immediately
                    // without showing the overlay. This is useful when you
                    // have memorized all key mappings and don't need visual
                    // feedback anymore.
                    logger::info("Silent mode: overlay suppressed.");
                    // The overlay still needs focus to receive the next key,
                    // but we make it invisible (1x1 off-screen).
                    show_overlay_silent(hwnd);
                }
            } else {
                // Normal window messages (paint, keyboard, focus, etc.)
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    // Clean up: remove tray icon and unregister hotkey so the OS doesn't
    // hold resources for a dead process.
    tray::remove_tray(hwnd);
    hotkey::unregister(hwnd);
    Ok(())
}
