use std::io::prelude::*;
use std::io::BufReader;
use std::fs;
use std::string::FromUtf8Error;
use std::path::PathBuf;
use super::file::File;



#[tauri::command]
pub fn open_and_read_buffer(file_path: &str) -> Result<String, String> {
    let file = fs::File::open(file_path).map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(file);
    let mut buffer: Vec<u8> = Vec::new();
    reader.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
    let s = String::from_utf8(buffer).map_err(|e| e.to_string())?;
    Ok(s)
}

#[tauri::command]
pub fn open_directory(dir_path: String) -> Result<Vec<File>, String> {
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

