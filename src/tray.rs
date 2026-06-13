use std::path::PathBuf;
use windows::{
    core::*,
    Win32::{
        Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, WPARAM, POINT},
        Graphics::Gdi::{
            CreateCompatibleBitmap, CreateCompatibleDC, CreateSolidBrush, DeleteDC, DeleteObject,
            FillRect, GetDC, ReleaseDC, SelectObject,
        },
        System::LibraryLoader::{GetModuleFileNameW, GetModuleHandleW},
        UI::Shell::{
            Shell_NotifyIconW, NOTIFYICONDATAW, NIM_ADD, NIM_DELETE, NIF_ICON, NIF_MESSAGE, NIF_TIP,
        },
        UI::WindowsAndMessaging::{
            AppendMenuW, CreateIconIndirect, CreatePopupMenu, DestroyIcon, DestroyMenu, GetCursorPos,
            LoadImageW, SetForegroundWindow, TrackPopupMenu, HICON, ICONINFO, IMAGE_FLAGS, IMAGE_ICON, LR_LOADFROMFILE,
            MF_SEPARATOR, MF_STRING, TPM_RIGHTBUTTON, WM_APP,
        },
    },
};

// Custom message ID the tray icon sends to our window on mouse events.
const TRAY_MSG_ID: u32 = WM_APP + 1;

// Menu item IDs
const ID_SHOW_OVERLAY: u32 = 1001;
const ID_RELOAD_CONFIG: u32 = 1002;
const ID_OPEN_CONFIG_FOLDER: u32 = 1003;
const ID_QUIT: u32 = 1004;

/// Returns the directory containing the running EXE.
fn exe_dir() -> Option<PathBuf> {
    unsafe {
        let mut buf = vec![0u16; 32767]; // MAX_PATH_EXTENDED
        let len = GetModuleFileNameW(None, &mut buf);
        if len == 0 {
            return None;
        }
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        PathBuf::from(path).parent().map(PathBuf::from)
    }
}

/// Tries to load the icon from the EXE's embedded resources (ID 101).
/// If the resource isn't found, falls back to loading `vwi.ico` from the
/// EXE directory, then to the programmatic orange square.
unsafe fn load_or_create_icon() -> HICON {
    let hinstance: HINSTANCE = unsafe { GetModuleHandleW(None).unwrap().into() };

    // Attempt 1: embedded resource (icon baked into the EXE by vwi.rc).
    // MAKEINTRESOURCE(101) is represented as a cast to a pointer value.
    let res = unsafe {
        LoadImageW(
            Some(hinstance),
            PCWSTR(101 as *const u16),
            IMAGE_ICON,
            0,
            0,
            IMAGE_FLAGS(0),
        )
    };
    if let Ok(icon) = res {
        return HICON(icon.0);
    }

    // Attempt 2: external vwi.ico next to the EXE.
    if let Some(dir) = exe_dir() {
        let ico_path = dir.join("vwi.ico");
        if ico_path.exists() {
            let wide: Vec<u16> = ico_path
                .to_string_lossy()
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let hicon = unsafe {
                LoadImageW(
                    None,
                    PCWSTR(wide.as_ptr()),
                    IMAGE_ICON,
                    0,
                    0,
                    LR_LOADFROMFILE,
                )
            };
            if let Ok(icon) = hicon {
                return HICON(icon.0);
            }
        }
    }

    // Attempt 3: programmatic orange square.
    unsafe { create_fallback_icon() }
}

/// Creates a 32x32 orange square icon programmatically so VWi works
/// even when no external .ico file is present.
unsafe fn create_fallback_icon() -> HICON {
    let width = 32i32;
    let height = 32i32;

    let hdc_screen = unsafe { GetDC(None) };
    let hdc_mem = unsafe { CreateCompatibleDC(Some(hdc_screen)) };

    // Color bitmap — filled with VWi accent orange (#FFA500).
    // COLORREF is 0x00BBGGRR, so FFA500 -> B=00, G=A5, R=FF -> 0x00A5FF.
    let hbm_color = unsafe { CreateCompatibleBitmap(hdc_screen, width, height) };
    let old_obj = unsafe { SelectObject(hdc_mem, hbm_color.into()) };
    let hbr = unsafe { CreateSolidBrush(COLORREF(0x00A5FF)) };
    let rect = windows::Win32::Foundation::RECT {
        left: 0,
        top: 0,
        right: width,
        bottom: height,
    };
    unsafe { let _ = FillRect(hdc_mem, &rect, hbr); }
    unsafe { let _ = DeleteObject(hbr.into()); }

    // Mask bitmap (monochrome, all white = fully opaque).
    let hbm_mask = unsafe { windows::Win32::Graphics::Gdi::CreateBitmap(width, height, 1, 1, None) };

    // Cleanup GDI objects.
    unsafe { let _ = SelectObject(hdc_mem, old_obj); }
    unsafe { let _ = DeleteDC(hdc_mem); }
    unsafe { let _ = ReleaseDC(None, hdc_screen); }

    // Build the icon from the two bitmaps.
    let info = ICONINFO {
        fIcon: windows_core::BOOL(1), // 1 = icon, 0 = cursor
        xHotspot: 0,
        yHotspot: 0,
        hbmMask: hbm_mask,
        hbmColor: hbm_color,
    };

    unsafe { CreateIconIndirect(&info).expect("CreateIconIndirect should succeed with valid bitmaps") }
}

/// Registers the system tray icon.
pub fn create_tray(hwnd: HWND) {
    unsafe {
        // Use vwi.ico from EXE directory if present; otherwise draw the
        // fallback orange square programmatically.
        let hicon = load_or_create_icon();

        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
            uCallbackMessage: TRAY_MSG_ID,
            hIcon: hicon,
            ..Default::default()
        };

        // Copy tooltip text into the fixed-size array.
        let tip = w!("VWi Window Switcher");
        let tip_slice = tip.as_wide();
        for (i, &ch) in tip_slice.iter().enumerate().take(127) {
            nid.szTip[i] = ch;
        }

        let _ = Shell_NotifyIconW(NIM_ADD, &mut nid);
        // Windows copied the icon bitmap internally; we can free our handle.
        let _ = DestroyIcon(hicon);
    }
}

/// Removes the tray icon on shutdown so it doesn't linger after exit.
pub fn remove_tray(hwnd: HWND) {
    unsafe {
        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1,
            ..Default::default()
        };
        let _ = Shell_NotifyIconW(NIM_DELETE, &mut nid);
    }
}

/// Returns true if the message is a tray-icon notification.
pub fn is_tray_msg(msg: u32) -> bool {
    msg == TRAY_MSG_ID
}

/// Returns true if the lParam indicates a right-button-up event.
pub fn is_rbutton_up(_lparam: LPARAM) -> bool {
    // WM_RBUTTONUP = 0x0205
    _lparam.0 as u32 == 0x0205
}

/// Displays the right-click context menu at the current cursor position.
pub fn show_tray_menu(hwnd: HWND) {
    unsafe {
        let hmenu = CreatePopupMenu().unwrap();
        let _ = AppendMenuW(hmenu, MF_STRING, ID_SHOW_OVERLAY as usize, w!("Show Overlay"));
        let _ = AppendMenuW(hmenu, MF_STRING, ID_RELOAD_CONFIG as usize, w!("Reload Config"));
        let _ = AppendMenuW(hmenu, MF_STRING, ID_OPEN_CONFIG_FOLDER as usize, w!("Open Config Folder"));
        let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, w!(""));
        let _ = AppendMenuW(hmenu, MF_STRING, ID_QUIT as usize, w!("Quit"));

        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);

        // Windows requires the window to be foreground before showing a popup menu.
        let _ = SetForegroundWindow(hwnd);

        let _ = TrackPopupMenu(
            hmenu,
            TPM_RIGHTBUTTON,
            pt.x,
            pt.y,
            Some(0),
            hwnd,
            None,
        );

        let _ = DestroyMenu(hmenu);
    }
}

/// Checks whether a WM_COMMAND message came from one of our tray menu items.
pub fn is_tray_menu_cmd(wparam: WPARAM) -> Option<TrayMenuAction> {
    match wparam.0 as u32 {
        ID_SHOW_OVERLAY => Some(TrayMenuAction::ShowOverlay),
        ID_RELOAD_CONFIG => Some(TrayMenuAction::ReloadConfig),
        ID_OPEN_CONFIG_FOLDER => Some(TrayMenuAction::OpenConfigFolder),
        ID_QUIT => Some(TrayMenuAction::Quit),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TrayMenuAction {
    ShowOverlay,
    ReloadConfig,
    OpenConfigFolder,
    Quit,
}
