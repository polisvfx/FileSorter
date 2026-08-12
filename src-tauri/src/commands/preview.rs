use super::matching::{compile_rules, RuleError};
use super::sort::{collect_files, resolve_conflict, resolve_destination};
use crate::models::Rule;
use serde::Serialize;
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
pub struct PreviewEntry {
    pub original_path: PathBuf,
    pub new_path: PathBuf,
    pub matched_rule_ids: Vec<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PreviewResult {
    pub entries: Vec<PreviewEntry>,
    /// Rules that would not compile — reported rather than silently skipped, so
    /// a half-typed regex shows up on the rule instead of just yielding no matches.
    pub rule_errors: Vec<RuleError>,
}

/// Resolve where every file would end up, without moving anything.
///
/// Conflicts are simulated against both what is already on disk and the
/// destinations claimed by earlier entries in this same run, so the names shown
/// here are the names a real sort would produce.
#[tauri::command]
pub fn preview_sort(
    paths: Vec<String>,
    rules: Vec<Rule>,
    output_dir: Option<String>,
) -> Result<PreviewResult, String> {
    let roots: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    let out_path = output_dir.map(PathBuf::from);

    let (compiled, rule_errors) = compile_rules(&rules);

    let mut entries = Vec::new();
    let mut claimed: HashSet<PathBuf> = HashSet::new();

    for file_path in collect_files(&roots) {
        let Some((dest, matched_rule_ids)) =
            resolve_destination(&file_path, &compiled, out_path.as_deref())
        else {
            continue;
        };

        let final_path = resolve_conflict(&dest, &claimed);
        claimed.insert(final_path.clone());

        entries.push(PreviewEntry {
            original_path: file_path,
            new_path: final_path,
            matched_rule_ids,
        });
    }

    Ok(PreviewResult {
        entries,
        rule_errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::sort::execute_sort;
    use crate::commands::sort::tests::rule;
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    /// The preview is only worth showing if it predicts exactly what the sort does,
    /// conflict suffixes included.
    #[test]
    fn preview_predicts_the_real_sort_exactly() {
        let dir = tempdir().unwrap();
        let src = dir.path();
        let out = dir.path().join("out");

        // Two files that collide in the target folder, plus one that matches nothing.
        for (sub, name) in [("a", "report.txt"), ("b", "report.txt"), ("", "notes.md")] {
            let parent = if sub.is_empty() { src.to_path_buf() } else { src.join(sub) };
            fs::create_dir_all(&parent).unwrap();
            fs::File::create(parent.join(name))
                .unwrap()
                .write_all(b"x")
                .unwrap();
        }

        let rules = vec![rule(1, "report", "Reports")];

        let preview = preview_sort(
            vec![src.to_string_lossy().to_string()],
            rules.clone(),
            Some(out.to_string_lossy().to_string()),
        )
        .unwrap();

        // Preview must not have touched anything.
        assert!(!out.exists(), "preview must not create directories");
        assert!(src.join("a").join("report.txt").exists());

        let result = execute_sort(vec![src.to_path_buf()], &rules, Some(out.clone()), false);
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        let predicted: Vec<_> = preview
            .entries
            .iter()
            .map(|e| (e.original_path.clone(), e.new_path.clone()))
            .collect();
        let actual: Vec<_> = result
            .operations
            .iter()
            .map(|o| (o.original_path.clone(), o.new_path.clone()))
            .collect();

        assert_eq!(predicted, actual, "preview diverged from the real sort");
        assert_eq!(preview.entries.len(), 2, "the non-matching file must not appear");
        assert!(preview.entries.iter().all(|e| e.matched_rule_ids == vec![1]));
        assert!(preview.rule_errors.is_empty());
    }

    #[test]
    fn preview_reports_every_matching_rule_in_order() {
        let dir = tempdir().unwrap();
        let src = dir.path();
        fs::File::create(src.join("clip_16x9_30s.mp4")).unwrap();

        let rules = vec![
            rule(7, "16x9", "16x9"),
            rule(9, "nomatch", "Nope"),
            rule(11, "_30s", "30s"),
        ];

        let preview =
            preview_sort(vec![src.to_string_lossy().to_string()], rules, None).unwrap();

        assert_eq!(preview.entries.len(), 1);
        assert_eq!(preview.entries[0].matched_rule_ids, vec![7, 11]);
        assert_eq!(
            preview.entries[0].new_path,
            src.join("16x9").join("30s").join("clip_16x9_30s.mp4")
        );
    }

    /// The headline reason tokens exist: one rule replaces a list of near-identical
    /// ones. `{ext}` alone sorts every format into its own folder.
    #[test]
    fn one_token_rule_replaces_a_rule_per_extension() {
        let dir = tempdir().unwrap();
        let src = dir.path();
        for name in ["a.mp4", "b.MOV", "c.wav", "d.mp4"] {
            fs::File::create(src.join(name)).unwrap();
        }

        let mut r = rule(1, ".", "{ext}");
        r.regex = true;

        let preview =
            preview_sort(vec![src.to_string_lossy().to_string()], vec![r], None).unwrap();

        assert!(preview.rule_errors.is_empty());
        assert_eq!(preview.entries.len(), 4);
        for (name, folder) in [("a.mp4", "mp4"), ("b.MOV", "mov"), ("c.wav", "wav")] {
            let entry = preview
                .entries
                .iter()
                .find(|e| e.original_path.ends_with(name))
                .unwrap_or_else(|| panic!("{name} missing from preview"));
            assert_eq!(entry.new_path, src.join(folder).join(name));
        }
    }

    #[test]
    fn a_broken_regex_is_reported_and_moves_nothing() {
        let dir = tempdir().unwrap();
        let src = dir.path();
        fs::File::create(src.join("clip.mp4")).unwrap();

        let mut r = rule(1, "([unclosed", "x");
        r.regex = true;

        let preview =
            preview_sort(vec![src.to_string_lossy().to_string()], vec![r], None).unwrap();

        assert!(preview.entries.is_empty());
        assert_eq!(preview.rule_errors.len(), 1);
        assert_eq!(preview.rule_errors[0].rule_id, 1);
    }
}
