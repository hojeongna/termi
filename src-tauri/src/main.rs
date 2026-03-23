// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// Termi 애플리케이션 진입점
fn main() {
    termi_lib::run()
}
