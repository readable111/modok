use std::io::prelude::*;
use std::io::BufReader;
use std::fs;
use std::string::FromUtf8Error;
use std::path::PathBuf;
use super::file::File;



#[tauri::command]
pub fn open_and_read_buffer(file_path: &str) -> Result<Vec<String>, String> {
    let file = fs::File::open(file_path).map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(file);
    let mut buffer: Vec<u8> = Vec::new();
    reader.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
    let mut lines: Vec<String> = Vec::new();
    let mut line_to_add: Vec<u8> = Vec::new();
    for ch in buffer.iter() {
        line_to_add.push(*ch);
        if *ch == b'\n' || *ch == b'\r' {
            let mut line = String::from_utf8(line_to_add.clone()).map_err(|e| e.to_string())?;
            lines.push(line);
            line_to_add.clear();
        }
    }

    Ok(lines)
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

