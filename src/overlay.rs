use crate::logger;
use crate::models::UiConfig;
use crate::switcher::Switcher;
use crate::tray;
use crate::windows::activate_window;
use std::cell::RefCell;
use std::rc::Rc;
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::LibraryLoader::GetModuleHandleW,
        UI::Shell::ShellExecuteW,
        UI::WindowsAndMessaging::*,
    },
};

// Thread-local storage lets the window procedure (a C-style callback)
// access application state without global variables.  We store an
// Rc<RefCell<AppState>> so the proc can clone a reference and safely
// borrow the Switcher when painting or handling keyboard input.
thread_local! {
    static APP_STATE: RefCell<Option<Rc<RefCell<AppState>>>> = RefCell::new(None);
}

/// Holds the shared Switcher instance and tracks overlay visibility.
/// `list_mode` is a temporary diagnostic view that shows ALL visible
/// windows (not just configured projects) so the user can discover
/// exact window titles for their config.
struct AppState {
    pub switcher: Rc<RefCell<Switcher>>,
    pub visible: bool,
    pub list_mode: bool,
    /// Vertical scroll offset in pixels. Only used when max_height is
    /// configured and the content exceeds the window size.
    pub scroll_offset: i32,
}

/// Creates a borderless, always-on-top popup window that serves as the
/// switcher overlay.  It starts hidden and is shown later by show_overlay().
///
/// Window style choices:
///   WS_EX_TOPMOST   - stays above all other windows
///   WS_EX_TOOLWINDOW - hides from Alt+Tab and the taskbar
///   WS_POPUP        - borderless, no caption bar
pub fn create_overlay(switcher: Rc<RefCell<Switcher>>) -> HWND {
    let class_name = w!("VWiOverlay");

    // GetModuleHandleW(None) returns the HINSTANCE of the current EXE.
    // We need it for RegisterClassW and CreateWindowExW.
    let hinstance = unsafe { GetModuleHandleW(None).unwrap() };

    let wc = WNDCLASSW {
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW).unwrap() },
        hInstance: hinstance.into(),
        lpszClassName: class_name,
        lpfnWndProc: Some(window_proc), // our custom message handler
        // hbrBackground is deliberately omitted — WM_PAINT fills the
        // entire client area with our custom color, so the default brush
        // (none) is never visible. This avoids a GDI brush leak.
        ..Default::default()
    };

    unsafe {
        let _ = RegisterClassW(&wc);
    }

    // Create the actual window.  It starts at a default size; show_overlay()
    // will resize it dynamically based on how many matches there are.
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            class_name,
            w!("VWi"),
            WS_POPUP,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            560,  // default initial width; show_overlay resizes dynamically
            300,
            None,               // no parent window
            None,               // no menu
            Some(hinstance.into()),
            None,
        )
        .unwrap()
    };

    // Store the state so window_proc can access the Switcher later.
    let state = Rc::new(RefCell::new(AppState {
        switcher,
        visible: false,
        list_mode: false,
        scroll_offset: 0,
    }));
    APP_STATE.with(|s| {
        *s.borrow_mut() = Some(state);
    });

    // Hide immediately; we only show it when the global hotkey is pressed.
    unsafe {
        let _ = ShowWindow(hwnd, SW_HIDE);
    }

    hwnd
}

/// Returns the work-area rectangle (screen area excluding the taskbar)
/// of the monitor that contains the currently focused window.
/// Falls back to the primary monitor if anything goes wrong.
fn foreground_monitor_work_area() -> RECT {
    unsafe {
        let hwnd = GetForegroundWindow();
        let hmonitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTOPRIMARY);
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            rcMonitor: RECT::default(),
            rcWork: RECT::default(),
            dwFlags: 0,
        };
        if GetMonitorInfoW(hmonitor, &mut mi).as_bool() {
            mi.rcWork
        } else {
            let cx = GetSystemMetrics(SM_CXSCREEN);
            let cy = GetSystemMetrics(SM_CYSCREEN);
            RECT {
                left: 0,
                top: 0,
                right: cx,
                bottom: cy,
            }
        }
    }
}

/// Shows the overlay centered on the monitor of the currently focused
/// window, sized to fit the current list of matching projects.
/// Also forces focus so keyboard input is captured immediately.
pub fn show_overlay(hwnd: HWND) {
    // Compute how many items we have so the window is never too tall or short,
    // and read the current UI config for sizing.
    let (match_count, ui) = APP_STATE.with(|s| {
        s.borrow().as_ref().map(|state| {
            let mut st = state.borrow_mut();
            st.list_mode = false; // always start in normal mode
            st.scroll_offset = 0; // reset scroll on show
            let switcher = st.switcher.borrow();
            let cfg = switcher.get_config();
            (switcher.matches().len(), cfg.ui.clone())
        }).unwrap_or((0, UiConfig::default()))
    });

    let content_height = ui.pad_y * 2 + (match_count as i32).max(1) * ui.line_height;
    let height = if ui.max_height > 0 {
        content_height.min(ui.max_height)
    } else {
        content_height
    };
    let width = ui.min_width;

    unsafe {
        let mon = foreground_monitor_work_area();
        let x = mon.left + (mon.right - mon.left - width) / 2;
        let y = mon.top + (mon.bottom - mon.top - height) / 2;
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            x,
            y,
            width,
            height,
            SWP_SHOWWINDOW | SWP_FRAMECHANGED,
        );
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
        let _ = InvalidateRect(Some(hwnd), None, true);
        let _ = UpdateWindow(hwnd);
    }
}

/// Shows the overlay in "silent" mode: the window is positioned
/// off-screen at 1x1 size and never painted, but it still receives
/// keyboard focus so the next key press triggers a project switch.
/// This is for users who have memorized all key mappings and don't
/// want any visual popup.
pub fn show_overlay_silent(hwnd: HWND) {
    APP_STATE.with(|s| {
        s.borrow().as_ref().map(|state| {
            state.borrow_mut().list_mode = false;
        });
    });
    unsafe {
        // Position far off-screen so the window is never visible.
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            -32000,
            -32000,
            1,
            1,
            SWP_SHOWWINDOW | SWP_FRAMECHANGED,
        );
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
        // Deliberately skip InvalidateRect/UpdateWindow so WM_PAINT
        // is never triggered and nothing is drawn.
    }
}

/// Hides the overlay and marks it as not visible.  Called on Escape,
/// focus loss, or after a successful project switch.
pub fn hide_overlay(hwnd: HWND) {
    let state_opt = APP_STATE.with(|s| s.borrow().as_ref().cloned());
    if let Some(state) = state_opt {
        state.borrow_mut().visible = false;
    }
    unsafe {
        let _ = ShowWindow(hwnd, SW_HIDE);
    }
}

/// The window procedure — Win32's callback-based event system.
/// Every message sent to our overlay window (paint, keyboard, focus, etc.)
/// arrives here.
///
/// CRITICAL DESIGN NOTE:
///   We must NEVER call Win32 APIs that can re-enter this proc
///   (e.g. SetForegroundWindow) while holding a RefCell borrow.
///   Windows may send WM_KILLFOCUS or WM_PAINT synchronously,
///   causing a panic.  That's why find_hwnd_by_key and activate_window
///   are split: we look up the HWND inside a minimal borrow scope,
///   drop the borrow, *then* call SetForegroundWindow.
unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        // WM_PAINT: draw the background, border, and list of matches.
        WM_PAINT => {
            unsafe {
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);

                let mut rect = RECT::default();
                let _ = GetClientRect(hwnd, &mut rect);
                let visible_height = rect.bottom - rect.top;

                // Read UI config so colors and sizes are customizable.
                let ui = APP_STATE.with(|s| {
                    s.borrow().as_ref().map(|state| {
                        let st = state.borrow();
                        let switcher = st.switcher.borrow();
                        switcher.get_config().ui.clone()
                    }).unwrap_or_default()
                });

                // Fill the entire client area with our dark background color.
                let hbr = CreateSolidBrush(COLORREF(ui.bg_color));
                let _ = FillRect(hdc, &rect, hbr);
                let _ = DeleteObject(hbr.into());

                // Draw a subtle 1px border so the popup doesn't blend into
                // other dark windows.
                let hpen = CreatePen(PS_SOLID, 1, COLORREF(ui.border_color));
                let old_pen = SelectObject(hdc, hpen.into());
                let old_brush = SelectObject(hdc, GetStockObject(NULL_BRUSH));
                let _ = Rectangle(hdc, rect.left, rect.top, rect.right, rect.bottom);
                let _ = SelectObject(hdc, old_brush);
                let _ = SelectObject(hdc, old_pen);
                let _ = DeleteObject(hpen.into());

                // Create normal and bold fonts for titles and keyboard keys.
                let hfont_normal = CreateFontW(
                    ui.font_height,
                    0,
                    0,
                    0,
                    FW_NORMAL.0 as i32,
                    0,
                    0,
                    0,
                    DEFAULT_CHARSET,
                    OUT_DEFAULT_PRECIS,
                    CLIP_DEFAULT_PRECIS,
                    DEFAULT_QUALITY,
                    (FIXED_PITCH.0 | FF_MODERN.0) as u32,
                    w!("Segoe UI"),
                );
                let hfont_bold = CreateFontW(
                    ui.font_height,
                    0,
                    0,
                    0,
                    FW_BOLD.0 as i32,
                    0,
                    0,
                    0,
                    DEFAULT_CHARSET,
                    OUT_DEFAULT_PRECIS,
                    CLIP_DEFAULT_PRECIS,
                    DEFAULT_QUALITY,
                    (FIXED_PITCH.0 | FF_MODERN.0) as u32,
                    w!("Segoe UI"),
                );
                let old_font = SelectObject(hdc, hfont_normal.into());
                let _ = SetBkMode(hdc, TRANSPARENT);

                // Render content: either configured project matches or the
                // diagnostic "all visible windows" list.
                let state_opt = APP_STATE.with(|s| s.borrow().as_ref().cloned());
                let mut content_height = 0i32;
                let mut scrollbar_needed = false;
                if let Some(state) = state_opt {
                    let st = state.borrow();
                    let switcher = st.switcher.borrow();
                    let cfg = switcher.get_config();
                    let ui = &cfg.ui;
                    let scroll_offset = st.scroll_offset;
                    let mut y = ui.pad_y;
                    let icon_size = ui.icon_size.max(0);
                    let icon_gap = if icon_size > 0 { 8 } else { 0 };

                    let item_count = if st.list_mode {
                        crate::windows::enumerate_visible_windows().len() + 1
                    } else {
                        switcher.matches().len()
                    };
                    content_height = ui.pad_y * 2 + (item_count as i32).max(1) * ui.line_height;
                    scrollbar_needed = ui.max_height > 0 && content_height > visible_height;

                    if st.list_mode {
                        // Diagnostic mode: show every visible window so the
                        // user can discover exact titles for their config.
                        let header = "All Visible Windows (press L to return)";
                        let header_y = y - scroll_offset;
                        if header_y + ui.line_height > 0 && header_y < visible_height {
                            let _ = SetTextColor(hdc, COLORREF(ui.key_color));
                            let header_wide: Vec<u16> = header.encode_utf16().collect();
                            let _ = TextOutW(hdc, ui.pad_x, header_y, &header_wide);
                        }
                        y += ui.line_height;

                        let all_windows = crate::windows::enumerate_visible_windows();
                        for (idx, win) in all_windows.iter().enumerate() {
                            let item_y = y - scroll_offset;
                            if item_y + ui.line_height > 0 && item_y < visible_height {
                                let mut content_x = ui.pad_x;

                                // Draw window icon if enabled
                                if icon_size > 0 {
                                    if let Some(icon) = crate::windows::get_window_icon(win.hwnd) {
                                        let icon_y = item_y + (ui.line_height - icon_size) / 2;
                                        let _ = DrawIconEx(
                                            hdc, content_x, icon_y, icon,
                                            icon_size, icon_size, 0, None, DI_NORMAL,
                                        );
                                    }
                                    content_x += icon_size + icon_gap;
                                }

                                let num_str = format!("{}.", idx + 1);
                                let _ = SetTextColor(hdc, COLORREF(ui.key_color));
                                let num_wide: Vec<u16> = num_str.encode_utf16().collect();
                                let _ = TextOutW(hdc, content_x, item_y, &num_wide);

                                let mut num_size = SIZE::default();
                                let _ = GetTextExtentPoint32W(hdc, &num_wide, &mut num_size);
                                let title_x = content_x + num_size.cx + 12;

                                let _ = SetTextColor(hdc, COLORREF(ui.text_color));
                                let title_wide: Vec<u16> = win.title.encode_utf16().collect();
                                let _ = TextOutW(hdc, title_x, item_y, &title_wide);
                            }
                            y += ui.line_height;
                        }
                    } else {
                        // Normal mode: show configured project matches.
                        let matches = switcher.matches();
                        if matches.is_empty() {
                            // Empty state
                            let msg1 = "No matching windows";
                            let msg2 = "Press L to see all visible windows";
                            let center_y = visible_height / 2 - ui.line_height;

                            let _ = SetTextColor(hdc, COLORREF(ui.text_color));
                            let wide1: Vec<u16> = msg1.encode_utf16().collect();
                            let mut size1 = SIZE::default();
                            let _ = GetTextExtentPoint32W(hdc, &wide1, &mut size1);
                            let x1 = (rect.right - rect.left - size1.cx) / 2;
                            let _ = TextOutW(hdc, x1, center_y, &wide1);

                            let _ = SetTextColor(hdc, COLORREF(ui.border_color));
                            let wide2: Vec<u16> = msg2.encode_utf16().collect();
                            let mut size2 = SIZE::default();
                            let _ = GetTextExtentPoint32W(hdc, &wide2, &mut size2);
                            let x2 = (rect.right - rect.left - size2.cx) / 2;
                            let _ = TextOutW(hdc, x2, center_y + ui.line_height, &wide2);
                        } else {
                            for (key, title, hwnd) in matches {
                                let item_y = y - scroll_offset;
                                if item_y + ui.line_height > 0 && item_y < visible_height {
                                    let mut content_x = ui.pad_x;

                                    // Draw window icon if enabled
                                    if icon_size > 0 {
                                        if let Some(icon) = crate::windows::get_window_icon(hwnd) {
                                            let icon_y = item_y + (ui.line_height - icon_size) / 2;
                                            let _ = DrawIconEx(
                                                hdc, content_x, icon_y, icon,
                                                icon_size, icon_size, 0, None, DI_NORMAL,
                                            );
                                        }
                                        content_x += icon_size + icon_gap;
                                    }

                                    // Measure key text with bold font
                                    let _ = SelectObject(hdc, hfont_bold.into());
                                    let key_wide: Vec<u16> = key.encode_utf16().collect();
                                    let mut key_size = SIZE::default();
                                    let _ = GetTextExtentPoint32W(hdc, &key_wide, &mut key_size);

                                    // Draw key box background
                                    let box_pad_x = 6;
                                    let box_pad_y = 3;
                                    let box_w = key_size.cx + box_pad_x * 2;
                                    let box_h = key_size.cy + box_pad_y * 2;
                                    let box_x = content_x;
                                    let box_y = item_y + (ui.line_height - box_h) / 2;

                                    let box_brush = CreateSolidBrush(COLORREF(ui.key_box_color));
                                    let box_rect = RECT {
                                        left: box_x,
                                        top: box_y,
                                        right: box_x + box_w,
                                        bottom: box_y + box_h,
                                    };
                                    let _ = FillRect(hdc, &box_rect, box_brush);
                                    let _ = DeleteObject(box_brush.into());

                                    // Draw key text centered in box
                                    let _ = SetTextColor(hdc, COLORREF(ui.key_color));
                                    let _ = TextOutW(hdc, box_x + box_pad_x, box_y + box_pad_y, &key_wide);

                                    // Switch back to normal font for title
                                    let _ = SelectObject(hdc, hfont_normal.into());

                                    // Draw the window title
                                    let title_x = box_x + box_w + 12;
                                    let _ = SetTextColor(hdc, COLORREF(ui.text_color));
                                    let title_wide: Vec<u16> = title.encode_utf16().collect();
                                    let _ = TextOutW(hdc, title_x, item_y, &title_wide);
                                }
                                y += ui.line_height;
                            }
                        }
                    }
                }

                // Draw scrollbar if content exceeds the window height.
                if scrollbar_needed {
                    let track_x = rect.right - 10;
                    let track_w = 6;
                    let track_top = ui.pad_y;
                    let track_bottom = visible_height - ui.pad_y;

                    // Track
                    let track_brush = CreateSolidBrush(COLORREF(ui.border_color));
                    let track_rect = RECT {
                        left: track_x,
                        top: track_top,
                        right: track_x + track_w,
                        bottom: track_bottom,
                    };
                    let _ = FillRect(hdc, &track_rect, track_brush);
                    let _ = DeleteObject(track_brush.into());

                    // Thumb
                    let scroll_offset = APP_STATE.with(|s| {
                        s.borrow().as_ref().map(|state| state.borrow().scroll_offset).unwrap_or(0)
                    });
                    let track_h = track_bottom - track_top;
                    let thumb_h = (track_h * visible_height / content_height.max(1))
                        .max(20)
                        .min(track_h);
                    let thumb_range = (track_h - thumb_h).max(1);
                    let scroll_range = (content_height - visible_height).max(1);
                    let thumb_y = track_top + scroll_offset * thumb_range / scroll_range;

                    let thumb_brush = CreateSolidBrush(COLORREF(ui.key_color));
                    let thumb_rect = RECT {
                        left: track_x,
                        top: thumb_y,
                        right: track_x + track_w,
                        bottom: thumb_y + thumb_h,
                    };
                    let _ = FillRect(hdc, &thumb_rect, thumb_brush);
                    let _ = DeleteObject(thumb_brush.into());
                }

                let _ = SelectObject(hdc, old_font);
                let _ = DeleteObject(hfont_normal.into());
                let _ = DeleteObject(hfont_bold.into());
                let _ = EndPaint(hwnd, &ps);
            }
            LRESULT(0)
        }

        // WM_KEYDOWN: handle Escape (dismiss) and project keys (switch).
        WM_KEYDOWN => {
            let vk = wparam.0 as u32; // virtual-key code

            // Escape dismisses the overlay without switching.
            if vk == 0x1B {
                hide_overlay(hwnd);
                return LRESULT(0);
            }

            // F5 reloads the configuration from disk without restarting.
            // Useful when the user edits config.toml while the app is running.
            if vk == 0x74 {
                // F5
                logger::info("F5 pressed — reloading config...");
                let reloaded = APP_STATE.with(|s| {
                    s.borrow().as_ref().map(|state| {
                        let st = state.borrow_mut();
                        let mut switcher = st.switcher.borrow_mut();
                        switcher.reload_config()
                    }).unwrap_or(false)
                });
                if reloaded {
                    logger::info("Config reloaded successfully.");
                } else {
                    logger::error("Config reload failed.");
                }
                // Force a repaint so any UI color/size changes take effect.
                unsafe {
                    let _ = InvalidateRect(Some(hwnd), None, true);
                    let _ = UpdateWindow(hwnd);
                }
                return LRESULT(0);
            }

            // 'L' toggles diagnostic list mode: shows ALL visible windows
            // with their titles so the user can discover exact names for
            // their config.  This does NOT interfere with a project key
            // bound to 'l' — that is checked first below.
            if vk == 0x4C {
                let (list_mode, all_count, ui) = APP_STATE.with(|s| {
                    s.borrow().as_ref().map(|state| {
                        let mut st = state.borrow_mut();
                        st.list_mode = !st.list_mode;
                        let switcher = st.switcher.borrow();
                        let cfg = switcher.get_config();
                        let count = if st.list_mode {
                            crate::windows::enumerate_visible_windows().len()
                        } else {
                            switcher.matches().len()
                        };
                        (st.list_mode, count, cfg.ui.clone())
                    }).unwrap_or((false, 0, UiConfig::default()))
                });
                logger::info(&format!(
                    "List mode toggled: {} ({} items)",
                    list_mode, all_count
                ));
                // Resize the window to fit the new item count and force repaint.
                let content_height = ui.pad_y * 2 + (all_count as i32).max(1) * ui.line_height;
                let height = if ui.max_height > 0 {
                    content_height.min(ui.max_height)
                } else {
                    content_height
                };
                let width = ui.min_width;
                unsafe {
                    let _ = SetWindowPos(
                        hwnd,
                        Some(HWND_TOPMOST),
                        0, 0,
                        width,
                        height,
                        SWP_FRAMECHANGED | SWP_NOMOVE,
                    );
                    let _ = InvalidateRect(Some(hwnd), None, true);
                    let _ = UpdateWindow(hwnd);
                }
                return LRESULT(0);
            }

            // Up/Down arrows scroll the overlay when max_height is set and
            // the content exceeds the window size.
            if vk == 0x26 || vk == 0x28 {
                let scrolled = APP_STATE.with(|s| {
                    s.borrow().as_ref().map(|state| {
                        let mut st = state.borrow_mut();
                        let (max_height, line_height, pad_y, item_count) = {
                            let switcher = st.switcher.borrow();
                            let ui = &switcher.get_config().ui;
                            let count = if st.list_mode {
                                crate::windows::enumerate_visible_windows().len() + 1
                            } else {
                                switcher.matches().len()
                            };
                            (ui.max_height, ui.line_height, ui.pad_y, count)
                        };
                        if max_height > 0 {
                            let content_height = pad_y * 2
                                + (item_count as i32).max(1) * line_height;
                            let visible_height = max_height;
                            if content_height > visible_height {
                                let scroll_range = (content_height - visible_height).max(1);
                                let delta = if vk == 0x26 { -line_height } else { line_height };
                                st.scroll_offset = (st.scroll_offset + delta)
                                    .clamp(0, scroll_range);
                                true
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    }).unwrap_or(false)
                });
                if scrolled {
                    unsafe {
                        let _ = InvalidateRect(Some(hwnd), None, true);
                        let _ = UpdateWindow(hwnd);
                    }
                }
                return LRESULT(0);
            }

            // Convert VK codes for letters and digits into Rust chars.
            // We only care about single alphanumeric keys; everything
            // else is ignored.
            let key_char = if vk >= 0x30 && vk <= 0x39 {
                (vk as u8) as char
            } else if vk >= 0x41 && vk <= 0x5A {
                (vk as u8) as char
            } else {
                '\0'
            };

            if key_char != '\0' {
                // Look up the HWND with a *very brief* borrow so we don't
                // hold the RefCell when SetForegroundWindow re-enters us.
                let target_hwnd = APP_STATE.with(|s| {
                    s.borrow().as_ref().and_then(|state| {
                        let st = state.borrow();
                        let switcher = st.switcher.borrow();
                        switcher.find_hwnd_by_key(key_char)
                    })
                });
                if let Some(target) = target_hwnd {
                    hide_overlay(hwnd);
                    activate_window(target);
                }
            }
            LRESULT(0)
        }

        // WM_MOUSEWHEEL: scroll the overlay when max_height is set and
        // the content exceeds the window size.
        WM_MOUSEWHEEL => {
            let delta = ((wparam.0 >> 16) as i16) as i32; // HIWORD of wparam
            let scrolled = APP_STATE.with(|s| {
                s.borrow().as_ref().map(|state| {
                    let mut st = state.borrow_mut();
                    let (max_height, line_height, pad_y, item_count) = {
                        let switcher = st.switcher.borrow();
                        let ui = &switcher.get_config().ui;
                        let count = if st.list_mode {
                            crate::windows::enumerate_visible_windows().len() + 1
                        } else {
                            switcher.matches().len()
                        };
                        (ui.max_height, ui.line_height, ui.pad_y, count)
                    };
                    if max_height > 0 {
                        let content_height = pad_y * 2
                            + (item_count as i32).max(1) * line_height;
                        let visible_height = max_height;
                        if content_height > visible_height {
                            let scroll_range = (content_height - visible_height).max(1);
                            // WHEEL_DELTA = 120; one notch ≈ 3 lines, we scroll 1 line per notch
                            let line_delta = line_height * delta / 120;
                            st.scroll_offset = (st.scroll_offset - line_delta)
                                .clamp(0, scroll_range);
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }).unwrap_or(false)
            });
            if scrolled {
                unsafe {
                    let _ = InvalidateRect(Some(hwnd), None, true);
                    let _ = UpdateWindow(hwnd);
                }
            }
            LRESULT(0)
        }

        // WM_KILLFOCUS: if the user clicks outside the overlay, dismiss it.
        WM_KILLFOCUS => {
            hide_overlay(hwnd);
            LRESULT(0)
        }

        // WM_COMMAND: tray menu item selection.
        WM_COMMAND => {
            if let Some(action) = tray::is_tray_menu_cmd(wparam) {
                match action {
                    tray::TrayMenuAction::ShowOverlay => {
                        logger::info("Tray: Show Overlay selected");
                        let show_ui = APP_STATE.with(|s| {
                            s.borrow().as_ref().map(|state| {
                                let st = state.borrow();
                                let switcher = st.switcher.borrow();
                                switcher.get_config().ui.show_overlay
                            }).unwrap_or(true)
                        });
                        if show_ui {
                            show_overlay(hwnd);
                        } else {
                            show_overlay_silent(hwnd);
                        }
                    }
                    tray::TrayMenuAction::ReloadConfig => {
                        logger::info("Tray: Reload Config selected");
                        let reloaded = APP_STATE.with(|s| {
                            s.borrow().as_ref().map(|state| {
                                let st = state.borrow_mut();
                                let mut switcher = st.switcher.borrow_mut();
                                switcher.reload_config()
                            }).unwrap_or(false)
                        });
                        if reloaded {
                            logger::info("Config reloaded successfully.");
                        } else {
                            logger::error("Config reload failed.");
                        }
                    }
                    tray::TrayMenuAction::OpenConfigFolder => {
                        logger::info("Tray: Open Config Folder selected");
                        let config_path = crate::config::config_path();
                        if let Some(config_dir) = config_path.parent() {
                            let path_wide: Vec<u16> = config_dir
                                .to_string_lossy()
                                .encode_utf16()
                                .chain(std::iter::once(0))
                                .collect();
                            unsafe {
                                let _ = ShellExecuteW(
                                    None,
                                    w!("open"),
                                    PCWSTR(path_wide.as_ptr()),
                                    None,
                                    None,
                                    SW_SHOWNORMAL,
                                );
                            }
                        } else {
                            logger::error("Could not determine config folder path.");
                        }
                    }
                    tray::TrayMenuAction::Quit => {
                        logger::info("Tray: Quit selected");
                        unsafe {
                            PostQuitMessage(0);
                        }
                    }
                }
            }
            LRESULT(0)
        }

        // Tray icon right-click: show the context menu.
        msg if tray::is_tray_msg(msg) => {
            if tray::is_rbutton_up(lparam) {
                tray::show_tray_menu(hwnd);
            }
            LRESULT(0)
        }

        // All other messages are handled by the default Windows procedure.
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
