//! Reads the file(s) currently selected in Explorer (Windows) / Finder (macOS).

use std::path::PathBuf;

/// Best-effort selected files. Empty vec = nothing selected or unsupported context.
pub fn current_selection() -> Vec<PathBuf> {
    #[cfg(windows)]
    return windows_selection();
    #[cfg(target_os = "macos")]
    return macos_selection();
    #[cfg(not(any(windows, target_os = "macos")))]
    return Vec::new();
}

// ---------------------------------------------------------------------------
// Windows: Explorer selection via COM/Shell Automation
// ---------------------------------------------------------------------------

#[cfg(windows)]
fn windows_selection() -> Vec<PathBuf> {
    use windows::{
        core::*,
        Win32::Foundation::*,
        Win32::System::Com::*,
        Win32::UI::Shell::*,
        Win32::UI::WindowsAndMessaging::*,
    };

    struct ComInit;
    impl ComInit {
        unsafe fn new() -> Self {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);
            ComInit
        }
    }
    impl Drop for ComInit {
        fn drop(&mut self) {
            unsafe { CoUninitialize() };
        }
    }

    unsafe fn extract_items_from_view(view: &IShellFolderViewDual) -> Vec<PathBuf> {
        let mut out = Vec::new();
        if let Ok(items) = view.SelectedItems() {
            if let Ok(count) = items.Count() {
                for i in 0..count {
                    if let Ok(item) = items.Item(&VARIANT::from(i)) {
                        if let Ok(path_bstr) = item.Path() {
                            let s = path_bstr.to_string();
                            if !s.is_empty() {
                                let p = PathBuf::from(s);
                                if p.exists() && p.is_file() {
                                    out.push(p);
                                }
                            }
                        }
                    }
                }
            }
        }
        out
    }

    unsafe fn inner() -> Result<Vec<PathBuf>> {
        let _com = ComInit::new();

        let fg = GetForegroundWindow();
        if fg.0.is_null() {
            return Ok(Vec::new());
        }

        let fg_root = GetAncestor(fg, GA_ROOT);

        let shell_windows: IShellWindows = CoCreateInstance(&ShellWindows, None, CLSCTX_ALL)?;
        let count = shell_windows.Count()?;

        // Windows 11 Explorer with tabs:
        // Find the active tab HWND (topmost ShellTabWindowClass in Z-order).
        let class_name: Vec<u16> = "ShellTabWindowClass\0".encode_utf16().collect();
        let active_tab = FindWindowExW(fg_root, None, PCWSTR(class_name.as_ptr()), None)
            .unwrap_or(HWND(std::ptr::null_mut()));

        const SID_S_TOP_LEVEL_BROWSER: GUID = GUID::from_u128(0x4C96BE40_915C_11CF_99D3_00AA004AE837);

        // Pass 1: If active tab is identified, find its exact IWebBrowser2 instance via IServiceProvider
        if !active_tab.0.is_null() {
            for i in 0..count {
                if let Ok(dispatch) = shell_windows.Item(&VARIANT::from(i)) {
                    if let Ok(sp) = dispatch.cast::<IServiceProvider>() {
                        if let Ok(sb) = sp.QueryService::<IShellBrowser>(&SID_S_TOP_LEVEL_BROWSER) {
                            if let Ok(tab_hwnd) = sb.GetWindow() {
                                if tab_hwnd == active_tab {
                                    if let Ok(browser) = dispatch.cast::<IWebBrowser2>() {
                                        if let Ok(doc_disp) = browser.Document() {
                                            if let Ok(view) = doc_disp.cast::<IShellFolderViewDual>() {
                                                let paths = extract_items_from_view(&view);
                                                return Ok(paths);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Pass 2: Foreground Explorer window (match win_root == fg_root or win_hwnd == fg_root)
        for i in 0..count {
            let dispatch: IDispatch = match shell_windows.Item(&VARIANT::from(i)) {
                Ok(d) => d,
                Err(_) => continue,
            };

            if let Ok(browser) = dispatch.cast::<IWebBrowser2>() {
                let win_hwnd = if let Ok(h) = browser.HWND() {
                    HWND(h.0 as *mut _)
                } else {
                    HWND(std::ptr::null_mut())
                };

                let win_root = if !win_hwnd.0.is_null() {
                    GetAncestor(win_hwnd, GA_ROOT)
                } else {
                    HWND(std::ptr::null_mut())
                };

                if win_hwnd == fg || win_hwnd == fg_root || win_root == fg_root {
                    if let Ok(doc_disp) = browser.Document() {
                        if let Ok(view) = doc_disp.cast::<IShellFolderViewDual>() {
                            let paths = extract_items_from_view(&view);
                            if !paths.is_empty() {
                                return Ok(paths);
                            }
                        }
                    }
                }
            }
        }

        // Pass 3: Check Desktop selection if Desktop is focused
        let mut desktop_hwnd = 0i32;
        if let Ok(disp) = shell_windows.FindWindowSW(
            &VARIANT::from(CSIDL_DESKTOP as i32),
            &VARIANT::default(),
            SWC_DESKTOP,
            &mut desktop_hwnd,
            SWFO_NEEDDISPATCH,
        ) {
            let desk_h = HWND(desktop_hwnd as *mut _);
            let desk_root = if !desk_h.0.is_null() {
                GetAncestor(desk_h, GA_ROOT)
            } else {
                HWND(std::ptr::null_mut())
            };
            if desk_h == fg || desk_h == fg_root || desk_root == fg_root {
                if let Ok(browser) = disp.cast::<IWebBrowser2>() {
                    if let Ok(doc) = browser.Document() {
                        if let Ok(view) = doc.cast::<IShellFolderViewDual>() {
                            let desktop_items = extract_items_from_view(&view);
                            if !desktop_items.is_empty() {
                                return Ok(desktop_items);
                            }
                        }
                    }
                }
            }
        }

        Ok(Vec::new())
    }

    unsafe { inner().unwrap_or_default() }
}

// ---------------------------------------------------------------------------
// macOS: Finder selection via AppleScript
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
fn macos_selection() -> Vec<PathBuf> {
    use std::process::Command;

    let script = r#"
        tell application "Finder"
            set sel to selection as alias list
            set out to ""
            repeat with itemRef in sel
                set out to out & (POSIX path of itemRef) & linefeed
            end repeat
            return out
        end tell
    "#;

    let Ok(output) = Command::new("osascript")
        .arg("-e")
        .arg(script)
    else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .collect()
}
