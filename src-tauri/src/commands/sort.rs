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

/// Split a Contains/Contains-NOT expression into its OR groups of AND terms,
/// dropping empty terms. `a,b*c` becomes `[[a], [b, c]]`.
fn parse_expr(expr: &str) -> Vec<Vec<String>> {
    expr.split(',')
        .map(|group| {
            group
                .split('*')
                .map(|t| t.trim().to_lowercase())
                .filter(|t| !t.is_empty())
                .collect::<Vec<String>>()
        })
        .filter(|terms| !terms.is_empty())
        .collect()
}

/// Evaluate a Contains/Contains-NOT expression against a (lowercased) filename.
/// "," is OR, "*" is AND, and AND binds tighter than OR: `a,b*c` means `a OR (b AND c)`.
/// An expression with no searchable terms matches nothing — otherwise a rule of
/// bare operators would sweep up every file in the tree.
fn expr_matches(filename_lower: &str, expr: &str) -> bool {
    parse_expr(expr)
        .iter()
        .any(|terms| terms.iter().all(|t| filename_lower.contains(t.as_str())))
}

/// Check if a file matches a rule (case-insensitive).
fn matches_rule(filename: &str, rule: &Rule) -> bool {
    let lower = filename.to_lowercase();
    if !expr_matches(&lower, &rule.contains) {
        return false;
    }
    if let Some(ref not) = rule.contains_not {
        if expr_matches(&lower, not) {
            return false;
        }
    }
    true
}

/// Turn a Contains expression into a filesystem-safe folder name for the
/// target-folder auto-fallback, e.g. "invoice*2024" -> "invoice 2024".
/// Needed because "*" is a reserved character in Windows folder names.
fn sanitize_folder_name(expr: &str) -> String {
    expr.split([',', '*'])
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Resolve where a file ends up after every rule has been applied, along with the
/// ids of the rules that matched it. Returns `None` when the file stays put.
///
/// Touches the filesystem only through the caller's conflict resolution, so this
/// is safe to run for a preview.
///
/// Each matching rule appends one folder segment, and the segments nest. The base
/// is the output directory when one is set, otherwise the file's original parent —
/// so `16x9` then `30s` yields `16x9/30s/` either way.
pub(crate) fn resolve_destination(
    file_path: &Path,
    rules: &[Rule],
    output_dir: Option<&Path>,
) -> Option<(PathBuf, Vec<u32>)> {
    let filename = file_path.file_name()?.to_string_lossy().to_string();

    let mut segments: Vec<String> = Vec::new();
    let mut matched_rule_ids: Vec<u32> = Vec::new();

    for rule in rules {
        if !rule.enabled || !matches_rule(&filename, rule) {
            continue;
        }

        matched_rule_ids.push(rule.id);

        let effective_folder = if rule.target_folder.trim().is_empty() {
            sanitize_folder_name(&rule.contains)
        } else {
            rule.target_folder.trim().to_string()
        };
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

/// Execute sorting rules on the given paths.
pub fn execute_sort(
    roots: Vec<PathBuf>,
    rules: &[Rule],
    output_dir: Option<PathBuf>,
    copy_mode: bool,
) -> SortResult {
    let mut operations = Vec::new();
    let mut errors = Vec::new();
    let mut claimed: HashSet<PathBuf> = HashSet::new();

    for file_path in collect_files(&roots) {
        let Some((dest, _)) = resolve_destination(&file_path, rules, output_dir.as_deref()) else {
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

    SortResult { operations, errors }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    fn rule(id: u32, contains: &str, folder: &str) -> Rule {
        Rule {
            id,
            contains: contains.to_string(),
            contains_not: None,
            target_folder: folder.to_string(),
            enabled: true,
            stop_on_match: false,
        }
    }

    fn write_file(dir: &Path, name: &str) -> PathBuf {
        fs::create_dir_all(dir).unwrap();
        let path = dir.join(name);
        let mut f = File::create(&path).unwrap();
        f.write_all(b"content").unwrap();
        path
    }

    // --- expression parsing -------------------------------------------------

    #[test]
    fn or_matches_either_term() {
        assert!(expr_matches("receipt_2024.pdf", "invoice,receipt"));
        assert!(expr_matches("invoice_2024.pdf", "invoice,receipt"));
        assert!(!expr_matches("statement.pdf", "invoice,receipt"));
    }

    #[test]
    fn and_requires_every_term() {
        assert!(expr_matches("invoice_2024.pdf", "invoice*2024"));
        assert!(!expr_matches("invoice_2023.pdf", "invoice*2024"));
    }

    #[test]
    fn and_binds_tighter_than_or() {
        // `invoice*2024,receipt` means (invoice AND 2024) OR receipt
        let expr = "invoice*2024,receipt";
        assert!(expr_matches("invoice_2024.pdf", expr));
        assert!(expr_matches("receipt_2023.pdf", expr));
        assert!(!expr_matches("invoice_2023.pdf", expr));
    }

    #[test]
    fn terms_are_trimmed() {
        assert!(expr_matches("invoice.pdf", " invoice , receipt "));
        assert!(expr_matches("invoice_2024.pdf", " invoice * 2024 "));
    }

    /// Regression: an expression of bare operators used to match *everything*,
    /// which swept every file in the tree into a folder.
    #[test]
    fn operator_only_expression_matches_nothing() {
        for expr in ["", "   ", "*", ",", ",,", "*,*", " , * "] {
            assert!(
                !expr_matches("anything.txt", expr),
                "expression {expr:?} should match nothing"
            );
            assert!(parse_expr(expr).is_empty(), "expression {expr:?} has no terms");
        }
        assert!(!parse_expr("invoice").is_empty());
    }

    // --- rule matching ------------------------------------------------------

    #[test]
    fn matching_is_case_insensitive() {
        let r = rule(1, "INVOICE", "Invoices");
        assert!(matches_rule("Invoice_2024.PDF", &r));
    }

    #[test]
    fn contains_not_excludes() {
        let mut r = rule(1, "invoice", "Invoices");
        r.contains_not = Some("draft".to_string());
        assert!(matches_rule("invoice_final.pdf", &r));
        assert!(!matches_rule("invoice_draft.pdf", &r));
    }

    #[test]
    fn empty_contains_not_does_not_exclude() {
        for not in ["", "   ", "*"] {
            let mut r = rule(1, "invoice", "Invoices");
            r.contains_not = Some(not.to_string());
            assert!(
                matches_rule("invoice.pdf", &r),
                "contains_not {not:?} should not exclude"
            );
        }
    }

    #[test]
    fn operator_only_rule_matches_no_file() {
        let r = rule(1, "*", "Everything");
        assert!(!matches_rule("anything.txt", &r));
        assert_eq!(
            resolve_destination(Path::new("/src/anything.txt"), &[r], None),
            None
        );
    }

    // --- folder name fallback ----------------------------------------------

    #[test]
    fn folder_name_strips_operators() {
        assert_eq!(sanitize_folder_name("invoice*2024"), "invoice 2024");
        assert_eq!(sanitize_folder_name("invoice,receipt"), "invoice receipt");
        assert_eq!(sanitize_folder_name("  spaced  "), "spaced");
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

        let (dest, ids) = resolve_destination(&file, &rules, None).unwrap();
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

        let (dest, _) = resolve_destination(&file, &rules, Some(&out)).unwrap();
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

        let (dest, ids) = resolve_destination(&file, &rules, None).unwrap();
        assert_eq!(dest, PathBuf::from("/src").join("16x9").join("clip_16x9_30s.mp4"));
        assert_eq!(ids, vec![1]);
    }

    #[test]
    fn disabled_rules_are_skipped() {
        let file = PathBuf::from("/src/clip_16x9_30s.mp4");
        let mut first = rule(1, "16x9", "16x9");
        first.enabled = false;
        let rules = vec![first, rule(2, "_30s", "30s")];

        let (dest, ids) = resolve_destination(&file, &rules, None).unwrap();
        assert_eq!(dest, PathBuf::from("/src").join("30s").join("clip_16x9_30s.mp4"));
        assert_eq!(ids, vec![2]);
    }

    #[test]
    fn unmatched_file_stays_put() {
        let file = PathBuf::from("/src/notes.txt");
        let rules = vec![rule(1, "16x9", "16x9")];
        assert_eq!(resolve_destination(&file, &rules, None), None);
    }

    #[test]
    fn blank_target_folder_falls_back_to_contains() {
        let file = PathBuf::from("/src/invoice_2024.pdf");
        let rules = vec![rule(1, "invoice*2024", "")];

        let (dest, _) = resolve_destination(&file, &rules, None).unwrap();
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
