mod commands;
mod models;

use commands::cancel::{self, CancelState};
use commands::presets;
use commands::preview;
use commands::resolve;
use commands::sort::{execute_sort_with, SortProgress};
use commands::undo::{self, UndoState};
use models::{Rule, SortResult};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager};

const PROGRESS_EVENT: &str = "sort://progress";
/// Emitting per file floods the IPC on large trees; this is frequent enough to
/// look live without swamping the webview.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(80);

#[derive(Clone, Serialize)]
struct ProgressPayload {
    processed: usize,
    total: usize,
    current: String,
}

#[tauri::command]
async fn sort_files(
    app: tauri::AppHandle,
    paths: Vec<String>,
    rules: Vec<Rule>,
    output_dir: Option<String>,
    copy_mode: bool,
) -> Result<SortResult, String> {
    // Run on a blocking thread: a sync command would occupy the main thread for
    // the whole sort, so `cancel_sort` could never be delivered.
    tauri::async_runtime::spawn_blocking(move || {
        let cancel_flag = {
            let state = app.state::<CancelState>();
            state.reset();
            state.flag()
        };

        let root_paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
        let out_path = output_dir.map(PathBuf::from);

        let mut last_emit: Option<Instant> = None;
        let mut on_progress = |progress: SortProgress| {
            let done = progress.processed == progress.total;
            if !done && last_emit.is_some_and(|t| t.elapsed() < PROGRESS_INTERVAL) {
                return;
            }
            last_emit = Some(Instant::now());
            let _ = app.emit(
                PROGRESS_EVENT,
                ProgressPayload {
                    processed: progress.processed,
                    total: progress.total,
                    current: progress.current.to_string_lossy().to_string(),
                },
            );
        };

        let result = execute_sort_with(
            root_paths,
            &rules,
            out_path.clone(),
            copy_mode,
            &mut on_progress,
            &|| cancel_flag.load(Ordering::Relaxed),
        );

        // Store operations for undo, along with the output dir that bounds cleanup.
        let undo_state = app.state::<UndoState>();
        let mut record = undo_state.inner.lock().map_err(|e| e.to_string())?;
        record.operations = result.operations.clone();
        record.output_dir = out_path;

        Ok(result)
    })
    .await
    .map_err(|e| format!("Sort task failed: {}", e))?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .manage(UndoState::new())
        .manage(CancelState::new())
        .invoke_handler(tauri::generate_handler![
            sort_files,
            cancel::cancel_sort,
            preview::preview_sort,
            resolve::inspect_paths,
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
