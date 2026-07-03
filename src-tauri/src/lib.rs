// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use tauri::{Builder, Manager};
use std::sync::Mutex;
mod buffers;
mod file;
mod piece_table;
use crate::piece_table::PieceTree;
use std::path::PathBuf;

#[derive(Default)]
struct AppState {
    open_file: Option<PieceTree>,
    open_directory: Option<PathBuf>,
    cursor_position: Option<(usize, usize)>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            app.manage(Mutex::new(AppState::default()));
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            buffers::open_and_read_buffer,
            buffers::open_directory,
            buffers::get_offset_position,
            buffers::write_changes
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
