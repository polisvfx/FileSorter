use super::sort::move_file;
use crate::models::FileOperation;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Default)]
pub struct UndoRecord {
    pub operations: Vec<FileOperation>,
    /// Output directory the sort ran with, if any. Bounds directory cleanup so
    /// undo can never climb above the folder the sort was writing into.
    pub output_dir: Option<PathBuf>,
}

pub struct UndoState {
    pub inner: Mutex<UndoRecord>,
}

impl UndoState {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(UndoRecord::default()),
        }
    }
}

/// Remove directories the sort created and the undo has now emptied, walking up
/// from `start`. Stops at `boundary` (exclusive) and at the first non-empty
/// directory, so cleanup can never delete a folder the sort did not create.
fn remove_empty_ancestors(start: &Path, boundary: &Path) {
    let mut cursor = Some(start.to_path_buf());

    while let Some(dir) = cursor {
        if dir == boundary || !dir.starts_with(boundary) || !dir.is_dir() {
            break;
        }

        let is_empty = fs::read_dir(&dir)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false);
        if !is_empty || fs::remove_dir(&dir).is_err() {
            break;
        }

        cursor = dir.parent().map(Path::to_path_buf);
    }
}

#[tauri::command]
pub fn undo_last_sort(state: tauri::State<'_, UndoState>) -> Result<Vec<String>, String> {
    let mut record = state.inner.lock().map_err(|e| e.to_string())?;
    if record.operations.is_empty() {
        return Err("Nothing to undo".to_string());
    }

    let mut errors = Vec::new();

    // Reverse the operations in reverse order
    for op in record.operations.iter().rev() {
        if !op.new_path.exists() {
            errors.push(format!("File no longer exists: {}", op.new_path.display()));
            continue;
        }

        if op.copied {
            if let Err(e) = fs::remove_file(&op.new_path) {
                errors.push(format!(
                    "Failed to delete copy '{}': {}",
                    op.new_path.display(),
                    e
                ));
            }
        } else {
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

            if let Err(e) = move_file(&op.new_path, &op.original_path) {
                errors.push(format!(
                    "Failed to restore '{}': {}",
                    op.new_path.display(),
                    e
                ));
            }
        }
    }

    // Clean up the directory tree the sort created. Each file may have been moved
    // several levels deep (16x9/30s/), so walk up rather than checking only the
    // immediate parent.
    for op in record.operations.iter() {
        let boundary = match record.output_dir.as_deref() {
            Some(out) => out,
            None => match op.original_path.parent() {
                Some(p) => p,
                None => continue,
            },
        };
        if let Some(parent) = op.new_path.parent() {
            remove_empty_ancestors(parent, boundary);
        }
    }

    record.operations.clear();
    record.output_dir = None;
    Ok(errors)
}

#[tauri::command]
pub fn can_undo(state: tauri::State<'_, UndoState>) -> bool {
    state
        .inner
        .lock()
        .map(|record| !record.operations.is_empty())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    /// Regression: cleanup used to check only the immediate parent, so undoing a
    /// nested sort left the outer folder (16x9/) sitting there empty.
    #[test]
    fn removes_the_whole_empty_chain() {
        let dir = tempdir().unwrap();
        let boundary = dir.path();
        let deep = boundary.join("16x9").join("30s");
        fs::create_dir_all(&deep).unwrap();

        remove_empty_ancestors(&deep, boundary);

        assert!(!deep.exists(), "innermost folder should be gone");
        assert!(!boundary.join("16x9").exists(), "empty parent should be gone too");
        assert!(boundary.exists(), "boundary itself must survive");
    }

    #[test]
    fn stops_at_the_first_non_empty_directory() {
        let dir = tempdir().unwrap();
        let boundary = dir.path();
        let deep = boundary.join("16x9").join("30s");
        fs::create_dir_all(&deep).unwrap();
        File::create(boundary.join("16x9").join("keep.txt")).unwrap();

        remove_empty_ancestors(&deep, boundary);

        assert!(!deep.exists(), "empty leaf should be gone");
        assert!(boundary.join("16x9").exists(), "folder with a file must stay");
    }

    #[test]
    fn never_removes_the_boundary_itself() {
        let dir = tempdir().unwrap();
        let boundary = dir.path();

        remove_empty_ancestors(boundary, boundary);

        assert!(boundary.exists(), "an empty boundary must never be deleted");
    }

    #[test]
    fn refuses_to_climb_outside_the_boundary() {
        let dir = tempdir().unwrap();
        let boundary = dir.path().join("output");
        let outside = dir.path().join("elsewhere");
        fs::create_dir_all(&boundary).unwrap();
        fs::create_dir_all(&outside).unwrap();

        remove_empty_ancestors(&outside, &boundary);

        assert!(outside.exists(), "directories outside the boundary are off limits");
    }
}
