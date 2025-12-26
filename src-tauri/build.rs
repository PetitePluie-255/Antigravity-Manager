fn main() {
    // 只在 tauri-app feature 启用时运行 tauri_build
    #[cfg(feature = "tauri-app")]
    tauri_build::build();
}
