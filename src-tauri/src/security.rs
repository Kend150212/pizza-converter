//! Security & Anti-Tamper Subsystem
//! Protects application binary against reverse engineering, attached debuggers,
//! dynamic process hooks, and unauthorized bot manipulation.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

static TAMPER_DETECTED: AtomicBool = AtomicBool::new(false);

/// Get current application package version with build profile
#[tauri::command]
pub fn get_app_version() -> String {
    let ver = "1.0.4.0";
    #[cfg(debug_assertions)]
    {
        format!("v{} (Dev)", ver)
    }
    #[cfg(not(debug_assertions))]
    {
        format!("v{} (Protected Release)", ver)
    }
}

/// Perform active debugger inspection
#[cfg(windows)]
pub fn is_debugger_active() -> bool {
    use windows::Win32::Foundation::BOOL;
    use windows::Win32::System::Diagnostics::Debug::{CheckRemoteDebuggerPresent, IsDebuggerPresent};
    use windows::Win32::System::Threading::GetCurrentProcess;

    unsafe {
        if IsDebuggerPresent().as_bool() {
            return true;
        }
        let mut remote_debugger: BOOL = BOOL(0);
        let curr_proc = GetCurrentProcess();
        if CheckRemoteDebuggerPresent(curr_proc, &mut remote_debugger).is_ok()
            && remote_debugger.as_bool()
        {
            return true;
        }
    }
    false
}

#[cfg(not(windows))]
pub fn is_debugger_active() -> bool {
    false
}

/// Verify process integrity and anti-debugging protection.
/// If an attached debugger or reverse-engineering tool is detected
/// in production mode, immediately terminates execution.
pub fn init_anti_tamper_shield() {
    #[cfg(not(debug_assertions))]
    {
        // 1. Initial check at startup
        if is_debugger_active() {
            eprintln!("[SECURITY ALERT] Debugger / memory inspector detected. Terminating process to protect integrity.");
            std::process::exit(0xDEAD);
        }

        // 2. Background heartbeat thread continuously checking process integrity
        std::thread::spawn(|| {
            loop {
                std::thread::sleep(Duration::from_secs(4));
                if is_debugger_active() {
                    TAMPER_DETECTED.store(true, Ordering::SeqCst);
                    eprintln!("[SECURITY ALERT] Unauthorized process tampering detected. Terminating immediately.");
                    std::process::exit(0xDEAD);
                }
            }
        });
    }
}
