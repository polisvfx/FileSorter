use super::sort::collect_files;
use serde::Serialize;
use std::path::PathBuf;

/// A path the user dropped or picked, kept as-is rather than expanded.
///
/// Expanding a dropped folder into one entry per file used to put tens of
/// thousands of rows into the UI list and into the persisted session; the file
/// count is all the UI actually needs to show.
#[derive(Debug, Clone, Serialize)]
pub struct PathInfo {
    pub path: PathBuf,
    pub is_dir: bool,
    pub file_count: usize,
}

/// Describe each root, skipping anything that no longer exists.
#[tauri::command]
pub fn inspect_paths(paths: Vec<String>) -> Result<Vec<PathInfo>, String> {
    let mut infos = Vec::new();

    for raw in paths {
        let path = PathBuf::from(raw);
        if path.is_file() {
            infos.push(PathInfo {
                path,
                is_dir: false,
                file_count: 1,
            });
        } else if path.is_dir() {
            let file_count = collect_files(std::slice::from_ref(&path)).len();
            infos.push(PathInfo {
                path,
                is_dir: true,
                file_count,
            });
        }
    }

    Ok(infos)
}
