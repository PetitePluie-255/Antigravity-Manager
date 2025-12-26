// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(feature = "tauri-app")]
fn main() {
    antigravity_tools_lib::run()
}

#[cfg(not(feature = "tauri-app"))]
fn main() {
    eprintln!("此二进制需要 tauri-app feature。请使用 antigravity-server 运行 Web 服务器。");
    std::process::exit(1);
}
