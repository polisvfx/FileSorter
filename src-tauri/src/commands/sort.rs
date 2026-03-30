use crate::models::{FileOperation, Rule, SortResult};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Resolve filename conflicts by appending (1), (2), etc.
fn resolve_conflict(target: &Path) -> PathBuf {
    if !target.exists() {
        return target.to_path_buf();
    }

    let stem = target
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let ext = target.extension().map(|e| format!(".{}", e.to_string_lossy()));
    let parent = target.parent().unwrap();

    let mut counter = 1u32;
    loop {
        let new_name = match &ext {
            Some(ext) => format!("{} ({}){}", stem, counter, ext),
            None => format!("{} ({})", stem, counter),
        };
        let candidate = parent.join(&new_name);
        if !candidate.exists() {
            return candidate;
        }
        counter += 1;
    }
}

/// Collect all file paths under the given roots.
pub(crate) fn collect_files(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for root in roots {
        if root.is_file() {
            files.push(root.clone());
        } else if root.is_dir() {
            for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    files.push(entry.into_path());
                }
            }
        }
    }
    files
}

/// Check if a file matches a rule (case-insensitive).
fn matches_rule(filename: &str, rule: &Rule) -> bool {
    let lower = filename.to_lowercase();
    let contains_match = lower.contains(&rule.contains.to_lowercase());
    if !contains_match {
        return false;
    }
    if let Some(ref not) = rule.contains_not {
        let not_trimmed = not.trim();
        if !not_trimmed.is_empty() && lower.contains(&not_trimmed.to_lowercase()) {
            return false;
        }
    }
    true
}

/// Execute sorting rules on the given paths.
/// All rules are evaluated per file to resolve a final destination path before
/// any move/copy is performed. This ensures every matching rule is applied even
/// when multiple rules target the same file.
pub fn execute_sort(
    roots: Vec<PathBuf>,
    rules: &[Rule],
    output_dir: Option<PathBuf>,
    copy_mode: bool,
) -> SortResult {
    let mut operations = Vec::new();
    let mut errors = Vec::new();

    let files = collect_files(&roots);

    for file_path in files {
        // Walk all rules to resolve the final destination path.
        let mut current_path = file_path.clone();

        for rule in rules {
            let current_filename = match current_path.file_name() {
                Some(name) => name.to_string_lossy().to_string(),
                None => break,
            };

            if !matches_rule(&current_filename, rule) {
                continue;
            }

            let effective_folder = if rule.target_folder.trim().is_empty() {
                rule.contains.trim().to_string()
            } else {
                rule.target_folder.trim().to_string()
            };

            let target_dir = if let Some(ref out) = output_dir {
                out.join(&effective_folder)
            } else {
                let parent = match current_path.parent() {
                    Some(p) => p,
                    None => break,
                };
                parent.join(&effective_folder)
            };

            current_path = target_dir.join(&current_filename);
        }

        // No rules matched — leave the file in place.
        if current_path == file_path {
            continue;
        }

        // Create target directory if needed.
        let target_dir = match current_path.parent() {
            Some(p) => p.to_path_buf(),
            None => continue,
        };
        if let Err(e) = fs::create_dir_all(&target_dir) {
            errors.push(format!(
                "Failed to create directory '{}': {}",
                target_dir.display(),
                e
            ));
            continue;
        }

        let final_path = resolve_conflict(&current_path);

        if copy_mode {
            match fs::copy(&file_path, &final_path) {
                Ok(_) => {
                    operations.push(FileOperation {
                        original_path: file_path,
                        new_path: final_path,
                        copied: true,
                    });
                }
                Err(e) => {
                    errors.push(format!("Failed to copy '{}': {}", file_path.display(), e));
                }
            }
        } else {
            match fs::rename(&file_path, &final_path) {
                Ok(_) => {
                    operations.push(FileOperation {
                        original_path: file_path,
                        new_path: final_path,
                        copied: false,
                    });
                }
                Err(e) => {
                    errors.push(format!("Failed to move '{}': {}", file_path.display(), e));
                }
            }
        }
    }

    SortResult { operations, errors }
}
