mod commands;
mod models;

use commands::presets;
use commands::resolve;
use commands::sort::execute_sort;
use commands::undo::{self, UndoState};
use models::{Rule, SortResult};
use std::path::PathBuf;

#[tauri::command]
fn sort_files(
    paths: Vec<String>,
    rules: Vec<Rule>,
    output_dir: Option<String>,
    copy_mode: bool,
    state: tauri::State<'_, UndoState>,
) -> Result<SortResult, String> {
    let root_paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    let out_path = output_dir.map(PathBuf::from);
    let result = execute_sort(root_paths, &rules, out_path, copy_mode);

    // Store operations for undo
    let mut ops = state.operations.lock().map_err(|e| e.to_string())?;
    *ops = result.operations.clone();

    Ok(result)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .manage(UndoState::new())
        .invoke_handler(tauri::generate_handler![
            sort_files,
            resolve::resolve_paths,
            presets::save_preset,
            presets::load_preset,
            presets::list_presets,
            presets::delete_preset,
            undo::undo_last_sort,
            undo::can_undo,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
