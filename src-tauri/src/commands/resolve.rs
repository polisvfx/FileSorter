use super::sort::collect_files;
use std::path::PathBuf;

#[tauri::command]
pub fn resolve_paths(paths: Vec<String>) -> Result<Vec<String>, String> {
    let roots: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    let files = collect_files(&roots);
    Ok(files
        .into_iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect())
}
