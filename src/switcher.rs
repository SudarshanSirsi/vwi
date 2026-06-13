use crate::config::load_config;
use crate::models::{Config, WindowInfo};
use windows::Win32::Foundation::HWND;

/// The core matching engine.  Holds the user config plus a snapshot of
/// visible windows refreshed on every hotkey press.
pub struct Switcher {
    config: Config,
    cache: Vec<WindowInfo>,
}

impl Switcher {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            cache: Vec::new(),
        }
    }

    /// Re-scan the desktop for visible windows.  We do this every time
    /// the hotkey is pressed so newly opened or closed windows are reflected
    /// immediately without restarting the app.
    pub fn refresh(&mut self) {
        self.cache = crate::windows::enumerate_visible_windows();
    }

    /// Returns the list of projects whose `match` pattern is found in any
    /// currently visible window title.  Each entry is:
    ///   (shortcut_key, window_title, hwnd)
    ///
    /// The result is sorted by key so the overlay always shows items in
    /// a stable, predictable order.
    pub fn matches(&self) -> Vec<(&str, &str, HWND)> {
        let mut results = Vec::new();
        for (_name, proj) in &self.config.projects {
            for win in &self.cache {
                if win.title.contains(&proj.r#match) {
                    results.push((proj.key.as_str(), &win.title[..], win.hwnd));
                    break; // first match wins for this project
                }
            }
        }
        results.sort_by_key(|(key, _, _)| *key);
        results
    }

    /// Looks up the HWND associated with a pressed key, but does NOT
    /// call any Win32 activation API.  This is intentionally split from
    /// the actual activation so callers can drop the RefCell borrow
    /// before SetForegroundWindow re-enters the window proc.
    pub fn find_hwnd_by_key(&self, key: char) -> Option<HWND> {
        let key_str = key.to_string();
        for (_name, proj) in &self.config.projects {
            if proj.key.eq_ignore_ascii_case(&key_str) {
                for win in &self.cache {
                    if win.title.contains(&proj.r#match) {
                        return Some(win.hwnd);
                    }
                }
            }
        }
        None
    }

    pub fn get_config(&self) -> &Config {
        &self.config
    }

    /// Re-reads config.toml from disk and replaces the in-memory config.
    /// Returns true on success, false if the file is missing or malformed.
    /// The caller must hold a mutable borrow (via RefCell::borrow_mut).
    pub fn reload_config(&mut self) -> bool {
        match load_config() {
            Ok(new_config) => {
                self.config = new_config;
                true
            }
            Err(e) => {
                crate::logger::error(&format!("Config reload failed: {}", e));
                false
            }
        }
    }
}
