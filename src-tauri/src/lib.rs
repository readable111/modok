// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod buffers;
mod file;
mod piece_table;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            buffers::open_and_read_buffer,
            buffers::open_directory,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
