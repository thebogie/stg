//! Tauri shell for STG. Loads the Yew (Trunk) frontend from `front/web`.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    stg_tauri_lib::run()
}
