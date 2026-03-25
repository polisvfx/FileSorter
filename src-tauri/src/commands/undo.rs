use crate::models::FileOperation;
use std::fs;
use std::sync::Mutex;

pub struct UndoState {
    pub operations: Mutex<Vec<FileOperation>>,
}

impl UndoState {
    pub fn new() -> Self {
        Self {
            operations: Mutex::new(Vec::new()),
        }
    }
}

#[tauri::command]
pub fn undo_last_sort(state: tauri::State<'_, UndoState>) -> Result<Vec<String>, String> {
    let mut ops = state.operations.lock().map_err(|e| e.to_string())?;
    if ops.is_empty() {
        return Err("Nothing to undo".to_string());
    }

    let mut errors = Vec::new();

    // Reverse the operations in reverse order
    for op in ops.iter().rev() {
        if !op.new_path.exists() {
            errors.push(format!(
                "File no longer exists: {}",
                op.new_path.display()
            ));
            continue;
        }

        // Ensure original parent directory exists
        if let Some(parent) = op.original_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                errors.push(format!(
                    "Failed to create directory '{}': {}",
                    parent.display(),
                    e
                ));
                continue;
            }
        }

        if let Err(e) = fs::rename(&op.new_path, &op.original_path) {
            errors.push(format!(
                "Failed to restore '{}': {}",
                op.new_path.display(),
                e
            ));
        }
    }

    // Clean up empty directories that were created during sort
    // We do this by checking if target dirs are now empty
    for op in ops.iter() {
        if let Some(parent) = op.new_path.parent() {
            if parent.is_dir() {
                if let Ok(mut entries) = fs::read_dir(parent) {
                    if entries.next().is_none() {
                        let _ = fs::remove_dir(parent);
                    }
                }
            }
        }
    }

    ops.clear();
    Ok(errors)
}

#[tauri::command]
pub fn can_undo(state: tauri::State<'_, UndoState>) -> bool {
    state
        .operations
        .lock()
        .map(|ops| !ops.is_empty())
        .unwrap_or(false)
}
