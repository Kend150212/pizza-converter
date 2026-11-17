#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    std::panic::set_hook(Box::new(|info| {
        let log_p = dirs::cache_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join("pizza_panic.log");
        let _ = std::fs::write(&log_p, format!("PANIC: {:?}\n", info));
    }));

    convertx_lib::run();
}
