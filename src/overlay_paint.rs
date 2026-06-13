//! Paint logic for the VWi overlay window.
//!
//! Everything drawn inside `WM_PAINT` lives here so `overlay.rs` stays a
//! thin message dispatcher.  The public entry point is [`paint`]; all
//! other items are private helpers.
//!
//! Every function wraps its Win32 calls in a single `unsafe` block so
//! the file compiles cleanly under Rust 2024 (`unsafe_op_in_unsafe_fn`).

use crate::models::UiConfig;
use crate::switcher::Switcher;
use windows::{
    core::*,
    Win32::{
        Foundation::{COLORREF, HWND, RECT, SIZE},
        Graphics::Gdi::*,
        UI::WindowsAndMessaging::*,
    },
};

/// Paints the entire overlay contents into the provided device context.
///
/// This includes background fill, border, the item list (normal or list mode),
/// empty-state message, and scrollbar thumb.
pub fn paint(
    hdc: HDC,
    rect: &RECT,
    ui: &UiConfig,
    scroll_offset: i32,
    list_mode: bool,
    switcher: &Switcher,
    visible_height: i32,
) {
    unsafe {
        paint_background(hdc, rect, ui);

        // Create fonts used by both modes.
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

        let icon_size = ui.icon_size.max(0);

        let item_count = if list_mode {
            crate::windows::enumerate_visible_windows().len() + 1
        } else {
            switcher.matches().len()
        };
        let content_height = ui.pad_y * 2 + (item_count as i32).max(1) * ui.line_height;
        let scrollbar_needed = ui.max_height > 0 && content_height > visible_height;

        if list_mode {
            paint_list_mode(
                hdc, rect, ui, hfont_normal, hfont_bold,
                scroll_offset, visible_height, icon_size,
            );
        } else {
            paint_normal_mode(
                hdc, rect, ui, hfont_normal, hfont_bold,
                scroll_offset, visible_height, icon_size, switcher,
            );
        }

        if scrollbar_needed {
            paint_scrollbar(hdc, rect, ui, content_height, visible_height, scroll_offset);
        }

        let _ = SelectObject(hdc, old_font);
        let _ = DeleteObject(hfont_normal.into());
        let _ = DeleteObject(hfont_bold.into());
    }
}

// ── Background ─────────────────────────────────────────────────────

fn paint_background(hdc: HDC, rect: &RECT, ui: &UiConfig) {
    unsafe {
        let hbr = CreateSolidBrush(COLORREF(ui.bg_color));
        let _ = FillRect(hdc, rect, hbr);
        let _ = DeleteObject(hbr.into());

        let hpen = CreatePen(PS_SOLID, 1, COLORREF(ui.border_color));
        let old_pen = SelectObject(hdc, hpen.into());
        let old_brush = SelectObject(hdc, GetStockObject(NULL_BRUSH));
        let _ = Rectangle(hdc, rect.left, rect.top, rect.right, rect.bottom);
        let _ = SelectObject(hdc, old_brush);
        let _ = SelectObject(hdc, old_pen);
        let _ = DeleteObject(hpen.into());
    }
}

// ── List mode (diagnostic: all visible windows) ────────────────────

fn paint_list_mode(
    hdc: HDC,
    _rect: &RECT,
    ui: &UiConfig,
    hfont_normal: HFONT,
    _hfont_bold: HFONT,
    scroll_offset: i32,
    visible_height: i32,
    icon_size: i32,
) {
    unsafe {
        let _ = SelectObject(hdc, hfont_normal.into());

        let header = "All Visible Windows (press L to return)";
        let header_y = ui.pad_y - scroll_offset;
        if header_y + ui.line_height > 0 && header_y < visible_height {
            let _ = SetTextColor(hdc, COLORREF(ui.key_color));
            let header_wide: Vec<u16> = header.encode_utf16().collect();
            let _ = TextOutW(hdc, ui.pad_x, header_y, &header_wide);
        }

        let mut y = ui.pad_y + ui.line_height;
        let all_windows = crate::windows::enumerate_visible_windows();
        for (idx, win) in all_windows.iter().enumerate() {
            let item_y = y - scroll_offset;
            if item_y + ui.line_height > 0 && item_y < visible_height {
                let mut content_x = ui.pad_x;
                let center_y = item_y + ui.line_height / 2;

                // 1. Index number (vertically centred)
                let num_str = format!("{}.", idx + 1);
                let _ = SetTextColor(hdc, COLORREF(ui.key_color));
                let num_wide: Vec<u16> = num_str.encode_utf16().collect();
                let mut num_size = SIZE::default();
                let _ = GetTextExtentPoint32W(hdc, &num_wide, &mut num_size);
                let num_y = center_y - num_size.cy / 2;
                let _ = TextOutW(hdc, content_x, num_y, &num_wide);
                content_x += num_size.cx + crate::defaults::LIST_NUM_ICON_GAP;

                // 2. Icon + title
                let _ = draw_row_content(
                    hdc, content_x, item_y, ui.line_height,
                    icon_size, win.hwnd, &win.title, ui,
                );
            }
            y += ui.line_height;
        }
    }
}

// ── Normal mode (configured project matches) ───────────────────────

fn paint_normal_mode(
    hdc: HDC,
    rect: &RECT,
    ui: &UiConfig,
    hfont_normal: HFONT,
    hfont_bold: HFONT,
    scroll_offset: i32,
    visible_height: i32,
    icon_size: i32,
    switcher: &Switcher,
) {
    unsafe {
        let matches = switcher.matches();
        if matches.is_empty() {
            paint_empty_state(hdc, rect, ui, visible_height);
            return;
        }

        let mut y = ui.pad_y;
        for (key, title, hwnd) in matches {
            let item_y = y - scroll_offset;
            if item_y + ui.line_height > 0 && item_y < visible_height {
                let mut content_x = ui.pad_x;
                let center_y = item_y + ui.line_height / 2;

                // 1. Keyboard key badge
                let _ = SelectObject(hdc, hfont_bold.into());
                let key_wide: Vec<u16> = key.encode_utf16().collect();
                let mut key_size = SIZE::default();
                let _ = GetTextExtentPoint32W(hdc, &key_wide, &mut key_size);

                let box_pad_x = 6;
                let box_pad_y = 4;
                let box_w = key_size.cx + box_pad_x * 2;
                let box_h = key_size.cy + box_pad_y * 2;
                let box_x = content_x;
                let box_y = center_y - box_h / 2;
                let text_y = box_y + (box_h - key_size.cy) / 2;

                let box_brush = CreateSolidBrush(COLORREF(ui.key_box_color));
                let box_rect = RECT {
                    left: box_x,
                    top: box_y,
                    right: box_x + box_w,
                    bottom: box_y + box_h,
                };
                let _ = FillRect(hdc, &box_rect, box_brush);
                let _ = DeleteObject(box_brush.into());

                let _ = SetTextColor(hdc, COLORREF(ui.key_color));
                let _ = TextOutW(hdc, box_x + box_pad_x, text_y, &key_wide);

                content_x = box_x + box_w + crate::defaults::KEY_ICON_GAP;

                // 2. Icon + title
                let _ = SelectObject(hdc, hfont_normal.into());
                let _ = draw_row_content(
                    hdc, content_x, item_y, ui.line_height,
                    icon_size, hwnd, title, ui,
                );
            }
            y += ui.line_height;
        }
    }
}

// ── Empty state ────────────────────────────────────────────────────

fn paint_empty_state(hdc: HDC, rect: &RECT, ui: &UiConfig, visible_height: i32) {
    unsafe {
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
    }
}

// ── Scrollbar ──────────────────────────────────────────────────────

fn paint_scrollbar(
    hdc: HDC,
    rect: &RECT,
    ui: &UiConfig,
    content_height: i32,
    visible_height: i32,
    scroll_offset: i32,
) {
    unsafe {
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
}

// ── Shared row helper ──────────────────────────────────────────────

/// Draws a window icon (if enabled) followed by the title text.
/// Both are vertically centred within the row.
fn draw_row_content(
    hdc: HDC,
    mut content_x: i32,
    item_y: i32,
    line_height: i32,
    icon_size: i32,
    hwnd: HWND,
    title: &str,
    ui: &UiConfig,
) -> i32 {
    unsafe {
        let center_y = item_y + line_height / 2;

        // Window icon
        if icon_size > 0 {
            if let Some(icon) = crate::windows::get_window_icon(hwnd) {
                let icon_y = center_y - icon_size / 2;
                let _ = DrawIconEx(
                    hdc, content_x, icon_y, icon,
                    icon_size, icon_size, 0, None, DI_NORMAL,
                );
            }
            content_x += icon_size + crate::defaults::ICON_TITLE_GAP;
        }

        // Title, vertically centred
        let _ = SetTextColor(hdc, COLORREF(ui.text_color));
        let title_wide: Vec<u16> = title.encode_utf16().collect();
        let mut title_size = SIZE::default();
        let _ = GetTextExtentPoint32W(hdc, &title_wide, &mut title_size);
        let title_y = center_y - title_size.cy / 2;
        let _ = TextOutW(hdc, content_x, title_y, &title_wide);

        content_x
    }
}
