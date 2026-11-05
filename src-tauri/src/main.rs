#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    std::panic::set_hook(Box::new(|info| {
        let log_p = dirs::cache_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join("pizza_panic.log");
        let _ = std::fs::write(&log_p, format!("PANIC: {:?}\n", info));
    }));

    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--mcp") {
        convertx_lib::mcp_hub::run_stdio_server();
        return;
    }
    convertx_lib::run();
}
