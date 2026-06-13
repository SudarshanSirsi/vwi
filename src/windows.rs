use crate::models::WindowInfo;
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM},
        UI::WindowsAndMessaging::{
            EnumWindows, GetWindowTextLengthW, GetWindowTextW, IsIconic, IsWindowVisible, IsZoomed,
            SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOWMAXIMIZED,
        },
    },
};
use windows_core::BOOL;

/// Enumerates every visible top-level window on the desktop and returns
/// a Vec of WindowInfo structs containing the HWND and window title.
///
/// We use EnumWindows because there is no simpler Win32 API that gives us
/// *all* top-level windows in one call. The callback receives each window
/// handle plus our user-defined LPARAM (here, a pointer to the Vec).
pub fn enumerate_visible_windows() -> Vec<WindowInfo> {
    let mut windows = Vec::new();
    unsafe {
        let _ = EnumWindows(
            Some(enum_callback),
            LPARAM(&mut windows as *mut Vec<WindowInfo> as isize),
        );
    }
    windows
}

/// Callback passed to EnumWindows.  Windows calls this once per top-level window.
///
/// We skip invisible windows and windows with empty titles (background shells,
/// message-only windows, etc.).  For everything else we copy the title into
/// a UTF-16 buffer, convert to Rust String, and push it into the Vec that
/// was passed through the LPARAM pointer.
unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        // Filter: only windows the user can actually see.
        if !IsWindowVisible(hwnd).as_bool() {
            return BOOL(1); // keep enumerating
        }

        let len = GetWindowTextLengthW(hwnd);
        if len == 0 {
            return BOOL(1); // no title, skip
        }

        // Allocate a UTF-16 buffer big enough for the title plus a null terminator.
        let mut buffer = vec![0u16; (len + 1) as usize];
        let copied = GetWindowTextW(hwnd, &mut buffer);
        if copied > 0 {
            let title = String::from_utf16_lossy(&buffer[..copied as usize]);
            // Reconstruct the Vec reference from the raw LPARAM pointer.
            let windows = &mut *(lparam.0 as *mut Vec<WindowInfo>);
            windows.push(WindowInfo { hwnd, title });
        }

        BOOL(1) // returning TRUE continues enumeration
    }
}

/// Brings the target window to the foreground.
///
/// If the window is minimized we restore it, but we try to preserve the
/// user's previous maximized state:
///   - Minimized + was maximized -> SW_SHOWMAXIMIZED
///   - Minimized + normal size   -> SW_RESTORE
///   - Not minimized             -> just SetForegroundWindow
///
/// This avoids the frustration of a fullscreen VS Code window shrinking to
/// normal size every time you switch to it.
pub fn activate_window(hwnd: HWND) {
    unsafe {
        if IsIconic(hwnd).as_bool() {
            // Window is minimized. Restore it, preserving maximized state if applicable.
            if IsZoomed(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_SHOWMAXIMIZED);
            } else {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            }
        }
        let _ = SetForegroundWindow(hwnd);
    }
}
