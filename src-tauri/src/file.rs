use serde::{Deserialize, Serialize, de::value::StringDeserializer};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct File {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub extension: Option<String>,
}

impl File {
    pub fn from_dir_entry(entry: std::fs::DirEntry) -> Self {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = path.is_dir();
        let extension = path
            .extension()
            .map(|e| e.to_string_lossy().to_string());

        File { name, path, is_dir, extension }
    }
}
