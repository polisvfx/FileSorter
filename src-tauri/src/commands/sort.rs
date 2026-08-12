use super::matching::{
    compile_rules, expand_tokens, sanitize_folder_name, scope_text, CompiledRule,
};
use crate::models::{FileOperation, Rule, SortResult};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Resolve filename conflicts by appending (1), (2), etc.
/// A path counts as taken if it exists on disk *or* is already claimed by an
/// earlier file in this run, so a preview produces the same names a real sort will.
pub(crate) fn resolve_conflict(target: &Path, claimed: &HashSet<PathBuf>) -> PathBuf {
    let is_taken = |p: &Path| claimed.contains(p) || p.exists();

    if !is_taken(target) {
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
        if !is_taken(candidate.as_path()) {
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

/// Resolve where a file ends up after every rule has been applied, along with the
/// ids of the rules that matched it. Returns `None` when the file stays put.
///
/// Reads file metadata only when a date token needs it, and never writes, so this
/// is safe to run for a preview.
///
/// Each matching rule appends one folder segment, and the segments nest. The base
/// is the output directory when one is set, otherwise the file's original parent —
/// so `16x9` then `30s` yields `16x9/30s/` either way.
pub(crate) fn resolve_destination(
    file_path: &Path,
    rules: &[CompiledRule],
    output_dir: Option<&Path>,
) -> Option<(PathBuf, Vec<u32>)> {
    let filename = file_path.file_name()?.to_string_lossy().to_string();

    let mut segments: Vec<String> = Vec::new();
    let mut matched_rule_ids: Vec<u32> = Vec::new();

    for rule in rules {
        let haystack = scope_text(file_path, rule.scope);
        let Some(captures) = rule.matches(&haystack) else {
            continue;
        };

        matched_rule_ids.push(rule.id);

        let template = rule.target_folder.trim();
        let effective_folder = if template.is_empty() {
            sanitize_folder_name(&rule.contains_source)
        } else {
            expand_tokens(template, file_path, captures.as_ref())
        };
        let effective_folder = effective_folder.trim().to_string();
        if !effective_folder.is_empty() {
            segments.push(effective_folder);
        }

        if rule.stop_on_match {
            break;
        }
    }

    if matched_rule_ids.is_empty() {
        return None;
    }

    let mut dest = match output_dir {
        Some(out) => out.to_path_buf(),
        None => file_path.parent()?.to_path_buf(),
    };
    for segment in &segments {
        dest.push(segment);
    }
    dest.push(&filename);

    if dest == file_path {
        return None;
    }

    Some((dest, matched_rule_ids))
}

/// Windows reports ERROR_NOT_SAME_DEVICE, Unix reports EXDEV.
fn is_cross_device(err: &std::io::Error) -> bool {
    #[cfg(windows)]
    const CROSS_DEVICE: i32 = 17;
    #[cfg(not(windows))]
    const CROSS_DEVICE: i32 = 18;

    err.raw_os_error() == Some(CROSS_DEVICE)
}

/// Move a file, falling back to copy+delete across volume boundaries where
/// `fs::rename` cannot reach. Only the cross-device error is retried — falling
/// back on any error would turn a permission failure into a silent copy.
pub(crate) fn move_file(from: &Path, to: &Path) -> std::io::Result<()> {
    match fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(e) if is_cross_device(&e) => {
            fs::copy(from, to)?;
            fs::remove_file(from)
        }
        Err(e) => Err(e),
    }
}

/// How far along a sort is. Reported per file so the UI can show real progress
/// instead of an indeterminate spinner.
#[derive(Debug, Clone)]
pub struct SortProgress<'a> {
    pub processed: usize,
    pub total: usize,
    pub current: &'a Path,
}

/// Sort without progress or cancellation. Production always goes through
/// [`execute_sort_with`]; this keeps the hook-free call short in tests.
#[cfg(test)]
pub fn execute_sort(
    roots: Vec<PathBuf>,
    rules: &[Rule],
    output_dir: Option<PathBuf>,
    copy_mode: bool,
) -> SortResult {
    execute_sort_with(roots, rules, output_dir, copy_mode, &mut |_| {}, &|| false)
}

/// Execute sorting rules, reporting progress and honouring cancellation.
///
/// The hooks are plain closures rather than Tauri types so the engine stays
/// testable on its own. Cancelling stops before the next file: everything already
/// moved is still recorded, so undo can reverse a cancelled run.
pub fn execute_sort_with(
    roots: Vec<PathBuf>,
    rules: &[Rule],
    output_dir: Option<PathBuf>,
    copy_mode: bool,
    on_progress: &mut dyn FnMut(SortProgress),
    is_cancelled: &dyn Fn() -> bool,
) -> SortResult {
    let mut operations = Vec::new();
    let mut claimed: HashSet<PathBuf> = HashSet::new();
    let mut cancelled = false;

    // Compile once rather than per file, and surface any rule that failed rather
    // than silently dropping it.
    let (compiled, rule_errors) = compile_rules(rules);
    let mut errors: Vec<String> = rule_errors
        .into_iter()
        .map(|e| format!("Rule {} skipped — invalid pattern. {}", e.rule_id, e.message))
        .collect();

    let files = collect_files(&roots);
    let total = files.len();

    for (index, file_path) in files.into_iter().enumerate() {
        if is_cancelled() {
            cancelled = true;
            break;
        }

        on_progress(SortProgress {
            processed: index,
            total,
            current: &file_path,
        });

        let Some((dest, _)) = resolve_destination(&file_path, &compiled, output_dir.as_deref())
        else {
            continue;
        };

        let target_dir = match dest.parent() {
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

        let final_path = resolve_conflict(&dest, &claimed);
        claimed.insert(final_path.clone());

        let outcome = if copy_mode {
            fs::copy(&file_path, &final_path).map(|_| ())
        } else {
            move_file(&file_path, &final_path)
        };

        match outcome {
            Ok(()) => operations.push(FileOperation {
                original_path: file_path,
                new_path: final_path,
                copied: copy_mode,
            }),
            Err(e) => {
                let verb = if copy_mode { "copy" } else { "move" };
                errors.push(format!(
                    "Failed to {} '{}': {}",
                    verb,
                    file_path.display(),
                    e
                ));
            }
        }
    }

    if !cancelled {
        on_progress(SortProgress {
            processed: total,
            total,
            current: Path::new(""),
        });
    }

    SortResult {
        operations,
        errors,
        cancelled,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    pub(crate) fn rule(id: u32, contains: &str, folder: &str) -> Rule {
        Rule {
            id,
            contains: contains.to_string(),
            contains_not: None,
            target_folder: folder.to_string(),
            enabled: true,
            stop_on_match: false,
            regex: false,
            case_sensitive: false,
            scope: crate::models::MatchScope::Name,
        }
    }

    /// Compile for the tests that drive `resolve_destination` directly.
    fn compiled(rules: &[Rule]) -> Vec<CompiledRule> {
        let (compiled, errors) = compile_rules(rules);
        assert!(errors.is_empty(), "rules failed to compile: {errors:?}");
        compiled
    }

    fn write_file(dir: &Path, name: &str) -> PathBuf {
        fs::create_dir_all(dir).unwrap();
        let path = dir.join(name);
        let mut f = File::create(&path).unwrap();
        f.write_all(b"content").unwrap();
        path
    }

    #[test]
    fn operator_only_rule_matches_no_file() {
        let rules = compiled(&[rule(1, "*", "Everything")]);
        assert_eq!(
            resolve_destination(Path::new("/src/anything.txt"), &rules, None),
            None
        );
    }

    // --- conflict resolution ------------------------------------------------

    #[test]
    fn conflict_free_path_is_unchanged() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("report.txt");
        assert_eq!(resolve_conflict(&target, &HashSet::new()), target);
    }

    #[test]
    fn existing_file_gets_numbered_suffix() {
        let dir = tempdir().unwrap();
        let target = write_file(dir.path(), "report.txt");
        assert_eq!(
            resolve_conflict(&target, &HashSet::new()),
            dir.path().join("report (1).txt")
        );
    }

    #[test]
    fn claimed_paths_count_as_taken() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("report.txt");

        // Nothing on disk, but an earlier file in this run already claimed it.
        let claimed: HashSet<PathBuf> = [target.clone()].into_iter().collect();
        assert_eq!(
            resolve_conflict(&target, &claimed),
            dir.path().join("report (1).txt")
        );

        // On disk *and* claimed once -> skips to (2).
        write_file(dir.path(), "report.txt");
        let claimed: HashSet<PathBuf> = [dir.path().join("report (1).txt")].into_iter().collect();
        assert_eq!(
            resolve_conflict(&target, &claimed),
            dir.path().join("report (2).txt")
        );
    }

    #[test]
    fn extensionless_files_get_suffixed() {
        let dir = tempdir().unwrap();
        let target = write_file(dir.path(), "LICENSE");
        assert_eq!(
            resolve_conflict(&target, &HashSet::new()),
            dir.path().join("LICENSE (1)")
        );
    }

    // --- destination resolution --------------------------------------------

    #[test]
    fn rules_nest_without_output_dir() {
        let file = PathBuf::from("/src/clip_16x9_30s.mp4");
        let rules = vec![rule(1, "16x9", "16x9"), rule(2, "_30s", "30s")];

        let (dest, ids) = resolve_destination(&file, &compiled(&rules), None).unwrap();
        assert_eq!(
            dest,
            PathBuf::from("/src").join("16x9").join("30s").join("clip_16x9_30s.mp4")
        );
        assert_eq!(ids, vec![1, 2]);
    }

    /// Regression for the output-dir bug: every rule used to rejoin from the
    /// output root, so rule 2 replaced rule 1's folder and the tree went flat.
    #[test]
    fn rules_nest_with_output_dir() {
        let file = PathBuf::from("/src/clip_16x9_30s.mp4");
        let out = PathBuf::from("/out");
        let rules = vec![rule(1, "16x9", "16x9"), rule(2, "_30s", "30s")];

        let (dest, _) = resolve_destination(&file, &compiled(&rules), Some(&out)).unwrap();
        assert_eq!(
            dest,
            out.join("16x9").join("30s").join("clip_16x9_30s.mp4"),
            "output dir must nest the same way as sorting in place"
        );
    }

    #[test]
    fn stop_on_match_halts_later_rules() {
        let file = PathBuf::from("/src/clip_16x9_30s.mp4");
        let mut first = rule(1, "16x9", "16x9");
        first.stop_on_match = true;
        let rules = vec![first, rule(2, "_30s", "30s")];

        let (dest, ids) = resolve_destination(&file, &compiled(&rules), None).unwrap();
        assert_eq!(dest, PathBuf::from("/src").join("16x9").join("clip_16x9_30s.mp4"));
        assert_eq!(ids, vec![1]);
    }

    #[test]
    fn disabled_rules_are_skipped() {
        let file = PathBuf::from("/src/clip_16x9_30s.mp4");
        let mut first = rule(1, "16x9", "16x9");
        first.enabled = false;
        let rules = vec![first, rule(2, "_30s", "30s")];

        let (dest, ids) = resolve_destination(&file, &compiled(&rules), None).unwrap();
        assert_eq!(dest, PathBuf::from("/src").join("30s").join("clip_16x9_30s.mp4"));
        assert_eq!(ids, vec![2]);
    }

    #[test]
    fn unmatched_file_stays_put() {
        let file = PathBuf::from("/src/notes.txt");
        let rules = vec![rule(1, "16x9", "16x9")];
        assert_eq!(resolve_destination(&file, &compiled(&rules), None), None);
    }

    #[test]
    fn blank_target_folder_falls_back_to_contains() {
        let file = PathBuf::from("/src/invoice_2024.pdf");
        let rules = vec![rule(1, "invoice*2024", "")];

        let (dest, _) = resolve_destination(&file, &compiled(&rules), None).unwrap();
        assert_eq!(
            dest,
            PathBuf::from("/src").join("invoice 2024").join("invoice_2024.pdf")
        );
    }

    // --- end-to-end ---------------------------------------------------------

    #[test]
    fn sorts_readme_example_into_nested_tree() {
        let dir = tempdir().unwrap();
        let src = dir.path();
        for name in [
            "ClientName_CampaignA_16x9_30s_v01.mp4",
            "ClientName_CampaignA_16x9_15s_v01.mp4",
            "ClientName_CampaignA_9x16_30s_v01.mp4",
        ] {
            write_file(src, name);
        }

        let rules = vec![
            rule(1, "16x9", "16x9"),
            rule(2, "9x16", "9x16"),
            rule(3, "_30s", "30s"),
            rule(4, "_15s", "15s"),
        ];

        let result = execute_sort(vec![src.to_path_buf()], &rules, None, false);
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        assert_eq!(result.operations.len(), 3);

        assert!(src
            .join("16x9/30s/ClientName_CampaignA_16x9_30s_v01.mp4")
            .exists());
        assert!(src
            .join("16x9/15s/ClientName_CampaignA_16x9_15s_v01.mp4")
            .exists());
        assert!(src
            .join("9x16/30s/ClientName_CampaignA_9x16_30s_v01.mp4")
            .exists());
        assert!(!src.join("ClientName_CampaignA_16x9_30s_v01.mp4").exists());
    }

    #[test]
    fn copy_mode_leaves_the_original_in_place() {
        let dir = tempdir().unwrap();
        let src = dir.path();
        write_file(src, "invoice_2024.pdf");

        let rules = vec![rule(1, "invoice", "Invoices")];
        let result = execute_sort(vec![src.to_path_buf()], &rules, None, true);

        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        assert!(src.join("invoice_2024.pdf").exists(), "original must remain");
        assert!(src.join("Invoices/invoice_2024.pdf").exists());
        assert!(result.operations[0].copied);
    }

    // --- progress and cancellation -----------------------------------------

    #[test]
    fn reports_progress_up_to_the_total() {
        let dir = tempdir().unwrap();
        for name in ["a_report.txt", "b_report.txt", "c_report.txt"] {
            write_file(dir.path(), name);
        }
        let rules = vec![rule(1, "report", "Reports")];

        let mut seen: Vec<(usize, usize)> = Vec::new();
        let result = execute_sort_with(
            vec![dir.path().to_path_buf()],
            &rules,
            None,
            false,
            &mut |p| seen.push((p.processed, p.total)),
            &|| false,
        );

        assert!(!result.cancelled);
        assert_eq!(result.operations.len(), 3);
        assert!(seen.iter().all(|(_, total)| *total == 3));
        assert_eq!(seen.first().unwrap().0, 0);
        assert_eq!(seen.last().unwrap().0, 3, "must report reaching the total");
    }

    #[test]
    fn cancelling_stops_early_but_keeps_what_already_moved() {
        let dir = tempdir().unwrap();
        for name in ["a_report.txt", "b_report.txt", "c_report.txt"] {
            write_file(dir.path(), name);
        }
        let rules = vec![rule(1, "report", "Reports")];

        // Let two files through, then cancel.
        let checks = std::cell::Cell::new(0usize);
        let result = execute_sort_with(
            vec![dir.path().to_path_buf()],
            &rules,
            None,
            false,
            &mut |_| {},
            &|| {
                let n = checks.get();
                checks.set(n + 1);
                n >= 2
            },
        );

        assert!(result.cancelled, "result must record that it was cancelled");
        assert_eq!(
            result.operations.len(),
            2,
            "files moved before the cancel stay undoable"
        );
        assert_eq!(
            fs::read_dir(dir.path().join("Reports")).unwrap().count(),
            2
        );
    }

    /// `fs::rename` cannot cross volumes, so moves fall back to copy+delete.
    /// Set FILESORTER_TEST_OTHER_VOLUME to a writable directory on a *different*
    /// drive than the system temp dir to exercise it; skipped when unset.
    #[test]
    fn moves_across_volumes() {
        let Ok(other_volume) = std::env::var("FILESORTER_TEST_OTHER_VOLUME") else {
            return;
        };

        let src = tempdir().unwrap();
        let out = tempfile::Builder::new()
            .prefix("filesorter-xdev-")
            .tempdir_in(&other_volume)
            .unwrap();

        let original = write_file(src.path(), "invoice_2024.pdf");
        let rules = vec![rule(1, "invoice", "Invoices")];

        let result = execute_sort(
            vec![src.path().to_path_buf()],
            &rules,
            Some(out.path().to_path_buf()),
            false,
        );

        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        assert_eq!(result.operations.len(), 1);

        let moved = out.path().join("Invoices").join("invoice_2024.pdf");
        assert!(moved.exists(), "file should have landed on the other volume");
        assert!(!original.exists(), "source should be gone after a move");
        assert_eq!(fs::read_to_string(&moved).unwrap(), "content");
    }

    #[test]
    fn same_named_files_from_different_folders_get_suffixed() {
        let dir = tempdir().unwrap();
        let src = dir.path();
        let out = dir.path().join("out");
        write_file(&src.join("a"), "report.txt");
        write_file(&src.join("b"), "report.txt");

        let rules = vec![rule(1, "report", "Reports")];
        let result = execute_sort(vec![src.to_path_buf()], &rules, Some(out.clone()), false);

        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        assert_eq!(result.operations.len(), 2);
        assert!(out.join("Reports/report.txt").exists());
        assert!(out.join("Reports/report (1).txt").exists());
    }
}

