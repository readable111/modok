use std::io::prelude::*;
use std::io::BufReader;
use std::fs;
use std::string::FromUtf8Error;
use std::path::PathBuf;
use std::sync::Mutex;
use super::AppState;
use super::piece_table::PieceTree;
use super::file::File;

#[tauri::command]
pub fn open_and_read_buffer(file_path: &str, state: tauri::State<'_, Mutex<AppState>>) -> Result<String, String> {
    let mut state = state.lock().unwrap();
    let file = fs::File::open(file_path).map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(file);
    let mut buffer: Vec<u8> = Vec::new();
    reader.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
    
    let mut s: &str = std::str::from_utf8(&buffer).expect("invalid UTF 8");
    let mut piece_tree = PieceTree::new(s);
    state.open_file = Some(piece_tree);

    Ok(state.open_file.as_mut().unwrap().get_text())
}

#[tauri::command]
pub fn open_directory(dir_path: String, state: tauri::State<'_, Mutex<AppState>>) -> Result<Vec<File>, String> {
    let mut state = state.lock().unwrap();
    let entries = fs::read_dir(&dir_path)
        .map_err(|e| e.to_string())?;

    let files: Vec<File> = entries
        .filter_map(|e| {
            let e = e.ok()?;
            Some(File::from_dir_entry(e))
        })
    .collect();
    Ok(files)
}

#[tauri::command]
pub fn get_offset_position(offset: usize, state: tauri::State<'_, Mutex<AppState>>) -> Result<Option<usize, usize>> {
    let mut state = state.lock().unwrap();
    state.cursor_position = state.open_file.offset_to_position(offset);
    Ok(state.cursor_position)
}
