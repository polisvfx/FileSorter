use crate::models::{MatchScope, Rule};
use chrono::{DateTime, Datelike, Local};
use regex::Regex;
use serde::Serialize;
use std::path::Path;

/// A rule whose pattern has been parsed or compiled once, up front, rather than
/// per file. Compiling a regex inside the per-file loop would dominate runtime.
pub struct CompiledRule {
    pub id: u32,
    pub target_folder: String,
    pub contains_source: String,
    pub stop_on_match: bool,
    pub scope: MatchScope,
    case_sensitive: bool,
    contains: Matcher,
    contains_not: Option<Matcher>,
}

enum Matcher {
    /// OR-groups of AND-terms, already case-folded when matching is insensitive.
    Expr(Vec<Vec<String>>),
    Regex(Regex),
}

/// A rule that could not be compiled, reported so the UI can flag it rather than
/// silently dropping the rule.
#[derive(Debug, Clone, Serialize)]
pub struct RuleError {
    pub rule_id: u32,
    pub message: String,
}

/// Split a Contains/Contains-NOT expression into its OR groups of AND terms,
/// dropping empty terms. `a,b*c` becomes `[[a], [b, c]]`.
fn parse_expr(expr: &str, case_sensitive: bool) -> Vec<Vec<String>> {
    expr.split(',')
        .map(|group| {
            group
                .split('*')
                .map(|t| {
                    let t = t.trim();
                    if case_sensitive {
                        t.to_string()
                    } else {
                        t.to_lowercase()
                    }
                })
                .filter(|t| !t.is_empty())
                .collect::<Vec<String>>()
        })
        .filter(|terms| !terms.is_empty())
        .collect()
}

fn build_matcher(pattern: &str, rule: &Rule) -> Result<Matcher, String> {
    if rule.regex {
        // Case-insensitivity is expressed inside the pattern so the regex engine
        // handles Unicode folding rather than us lowercasing the haystack.
        let source = if rule.case_sensitive {
            pattern.to_string()
        } else {
            format!("(?i){}", pattern)
        };
        Regex::new(&source)
            .map(Matcher::Regex)
            .map_err(|e| e.to_string())
    } else {
        Ok(Matcher::Expr(parse_expr(pattern, rule.case_sensitive)))
    }
}

/// Compile rules once. Rules that fail to compile are omitted — they match
/// nothing — and reported in the returned errors.
pub fn compile_rules(rules: &[Rule]) -> (Vec<CompiledRule>, Vec<RuleError>) {
    let mut compiled = Vec::new();
    let mut errors = Vec::new();

    for rule in rules {
        if !rule.enabled {
            continue;
        }

        let contains = match build_matcher(&rule.contains, rule) {
            Ok(m) => m,
            Err(message) => {
                errors.push(RuleError {
                    rule_id: rule.id,
                    message: format!("Contains: {}", message),
                });
                continue;
            }
        };

        let contains_not = match rule.contains_not.as_deref() {
            Some(pattern) if !pattern.trim().is_empty() => {
                match build_matcher(pattern, rule) {
                    Ok(m) => Some(m),
                    Err(message) => {
                        errors.push(RuleError {
                            rule_id: rule.id,
                            message: format!("Contains NOT: {}", message),
                        });
                        continue;
                    }
                }
            }
            _ => None,
        };

        compiled.push(CompiledRule {
            id: rule.id,
            target_folder: rule.target_folder.clone(),
            contains_source: rule.contains.clone(),
            stop_on_match: rule.stop_on_match,
            scope: rule.scope,
            case_sensitive: rule.case_sensitive,
            contains,
            contains_not,
        });
    }

    (compiled, errors)
}

/// The text a rule tests against, per its scope.
pub fn scope_text(path: &Path, scope: MatchScope) -> String {
    let part = match scope {
        MatchScope::Name => path.file_name(),
        MatchScope::Stem => path.file_stem(),
        MatchScope::Extension => path.extension(),
        MatchScope::Path => Some(path.as_os_str()),
    };
    part.map(|p| p.to_string_lossy().to_string()).unwrap_or_default()
}

impl Matcher {
    /// `haystack_folded` is already lowercased when matching is insensitive.
    fn is_match(&self, haystack: &str, haystack_folded: &str) -> bool {
        match self {
            // An expression with no searchable terms matches nothing — otherwise a
            // rule of bare operators would sweep up every file in the tree.
            Matcher::Expr(groups) => groups
                .iter()
                .any(|terms| terms.iter().all(|t| haystack_folded.contains(t.as_str()))),
            Matcher::Regex(re) => re.is_match(haystack),
        }
    }
}

impl CompiledRule {
    /// Test a file, returning regex captures when the rule uses them, so folder
    /// tokens like `$1` can be filled in.
    pub fn matches<'t>(&self, haystack: &'t str) -> Option<Option<regex::Captures<'t>>> {
        let folded = if self.case_sensitive {
            haystack.to_string()
        } else {
            haystack.to_lowercase()
        };

        let captures = match &self.contains {
            Matcher::Expr(_) => {
                if !self.contains.is_match(haystack, &folded) {
                    return None;
                }
                None
            }
            Matcher::Regex(re) => Some(re.captures(haystack)?),
        };

        if let Some(not) = &self.contains_not {
            if not.is_match(haystack, &folded) {
                return None;
            }
        }

        Some(captures)
    }
}

/// Strip characters that are not legal in a path segment on Windows. Applied to
/// substituted token *values* only — the template itself may contain `/` on
/// purpose to build nested folders.
fn sanitize_segment(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

fn modified_date(path: &Path) -> Option<DateTime<Local>> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    Some(DateTime::<Local>::from(modified))
}

/// Expand `{name}`, `{stem}`, `{ext}`, `{YYYY}`, `{MM}`, `{DD}` and regex
/// captures `$1`..`$9` in a target-folder template.
///
/// One rule with `{ext}` replaces one rule per extension, which is what makes a
/// long list of near-identical rules collapse into a single row.
pub fn expand_tokens(template: &str, path: &Path, captures: Option<&regex::Captures>) -> String {
    if !template.contains('{') && !template.contains('$') {
        return template.to_string();
    }

    let mut out = template.to_string();

    if out.contains("{name}") {
        let v = sanitize_segment(&scope_text(path, MatchScope::Name));
        out = out.replace("{name}", &v);
    }
    if out.contains("{stem}") {
        let v = sanitize_segment(&scope_text(path, MatchScope::Stem));
        out = out.replace("{stem}", &v);
    }
    if out.contains("{ext}") {
        let v = sanitize_segment(&scope_text(path, MatchScope::Extension).to_lowercase());
        out = out.replace("{ext}", &v);
    }

    if out.contains("{YYYY}") || out.contains("{MM}") || out.contains("{DD}") {
        let date = modified_date(path);
        let (y, m, d) = match date {
            Some(dt) => (
                format!("{:04}", dt.year()),
                format!("{:02}", dt.month()),
                format!("{:02}", dt.day()),
            ),
            None => ("unknown".into(), "unknown".into(), "unknown".into()),
        };
        out = out.replace("{YYYY}", &y).replace("{MM}", &m).replace("{DD}", &d);
    }

    if let Some(caps) = captures {
        for i in 1..=9 {
            let token = format!("${}", i);
            if !out.contains(&token) {
                continue;
            }
            let value = caps.get(i).map(|m| m.as_str()).unwrap_or("");
            out = out.replace(&token, &sanitize_segment(value));
        }
    }

    out
}

/// Turn a Contains expression into a filesystem-safe folder name for the
/// target-folder auto-fallback, e.g. "invoice*2024" -> "invoice 2024".
pub fn sanitize_folder_name(expr: &str) -> String {
    let joined = expr
        .split([',', '*'])
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    sanitize_segment(&joined)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::sort::tests::rule;

    /// Compile one rule, asserting it built cleanly.
    fn one(r: Rule) -> CompiledRule {
        let (mut compiled, errors) = compile_rules(&[r]);
        assert!(errors.is_empty(), "unexpected compile errors: {errors:?}");
        compiled.pop().expect("rule should compile")
    }

    fn matches(r: Rule, filename: &str) -> bool {
        let scope = r.scope;
        let compiled = one(r);
        compiled
            .matches(&scope_text(Path::new(filename), scope))
            .is_some()
    }

    // --- expression operators -----------------------------------------------

    #[test]
    fn or_matches_either_term() {
        assert!(matches(rule(1, "invoice,receipt", "x"), "receipt_2024.pdf"));
        assert!(matches(rule(1, "invoice,receipt", "x"), "invoice_2024.pdf"));
        assert!(!matches(rule(1, "invoice,receipt", "x"), "statement.pdf"));
    }

    #[test]
    fn and_requires_every_term() {
        assert!(matches(rule(1, "invoice*2024", "x"), "invoice_2024.pdf"));
        assert!(!matches(rule(1, "invoice*2024", "x"), "invoice_2023.pdf"));
    }

    #[test]
    fn and_binds_tighter_than_or() {
        // `invoice*2024,receipt` means (invoice AND 2024) OR receipt
        let expr = "invoice*2024,receipt";
        assert!(matches(rule(1, expr, "x"), "invoice_2024.pdf"));
        assert!(matches(rule(1, expr, "x"), "receipt_2023.pdf"));
        assert!(!matches(rule(1, expr, "x"), "invoice_2023.pdf"));
    }

    #[test]
    fn terms_are_trimmed() {
        assert!(matches(rule(1, " invoice , receipt ", "x"), "invoice.pdf"));
        assert!(matches(rule(1, " invoice * 2024 ", "x"), "invoice_2024.pdf"));
    }

    /// Regression: an expression of bare operators used to match *everything*,
    /// which swept every file in the tree into a folder.
    #[test]
    fn operator_only_expression_matches_nothing() {
        for expr in ["", "   ", "*", ",", ",,", "*,*", " , * "] {
            assert!(
                !matches(rule(1, expr, "x"), "anything.txt"),
                "expression {expr:?} should match nothing"
            );
            assert!(
                parse_expr(expr, false).is_empty(),
                "expression {expr:?} has no terms"
            );
        }
        assert!(!parse_expr("invoice", false).is_empty());
    }

    #[test]
    fn contains_not_excludes() {
        let mut r = rule(1, "invoice", "x");
        r.contains_not = Some("draft".to_string());
        assert!(matches(r.clone(), "invoice_final.pdf"));
        assert!(!matches(r, "invoice_draft.pdf"));
    }

    #[test]
    fn empty_contains_not_does_not_exclude() {
        for not in ["", "   ", "*"] {
            let mut r = rule(1, "invoice", "x");
            r.contains_not = Some(not.to_string());
            assert!(
                matches(r, "invoice.pdf"),
                "contains_not {not:?} should not exclude"
            );
        }
    }

    // --- case sensitivity ---------------------------------------------------

    #[test]
    fn matching_is_case_insensitive_by_default() {
        assert!(matches(rule(1, "INVOICE", "x"), "Invoice_2024.PDF"));
    }

    #[test]
    fn case_sensitive_rules_respect_case() {
        let mut r = rule(1, "Invoice", "x");
        r.case_sensitive = true;
        assert!(matches(r.clone(), "Invoice_2024.pdf"));
        assert!(!matches(r, "invoice_2024.pdf"));
    }

    #[test]
    fn case_sensitive_regex_respects_case() {
        let mut r = rule(1, "^Invoice", "x");
        r.regex = true;
        r.case_sensitive = true;
        assert!(matches(r.clone(), "Invoice_2024.pdf"));
        assert!(!matches(r, "invoice_2024.pdf"));
    }

    // --- match scope --------------------------------------------------------

    #[test]
    fn scope_selects_the_right_part_of_the_path() {
        let p = Path::new("/media/raw/clip_final.MP4");
        assert_eq!(scope_text(p, MatchScope::Name), "clip_final.MP4");
        assert_eq!(scope_text(p, MatchScope::Stem), "clip_final");
        assert_eq!(scope_text(p, MatchScope::Extension), "MP4");
        assert_eq!(scope_text(p, MatchScope::Path), "/media/raw/clip_final.MP4");
    }

    /// Scoping to the extension is what makes "mp4" mean the format rather than
    /// any filename that happens to contain those letters.
    #[test]
    fn extension_scope_ignores_the_stem() {
        let mut r = rule(1, "mp4", "Video");
        r.scope = MatchScope::Extension;
        assert!(matches(r.clone(), "clip.mp4"));
        assert!(
            !matches(r, "mp4_notes.txt"),
            "a stem containing 'mp4' must not match the extension scope"
        );
    }

    #[test]
    fn path_scope_can_match_parent_folders() {
        let mut r = rule(1, "raw", "Raw");
        r.scope = MatchScope::Path;
        assert!(matches(r.clone(), "/media/raw/clip.mp4"));
        assert!(!matches(r, "/media/final/clip.mp4"));
    }

    // --- regex --------------------------------------------------------------

    #[test]
    fn regex_mode_matches_patterns() {
        let mut r = rule(1, r"^\d{4}_report", "x");
        r.regex = true;
        assert!(matches(r.clone(), "2024_report.pdf"));
        assert!(!matches(r, "report_2024.pdf"));
    }

    #[test]
    fn regex_operators_are_not_treated_as_and_or() {
        // In regex mode `a,b` is a literal comma, not an OR.
        let mut r = rule(1, "a,b", "x");
        r.regex = true;
        assert!(matches(r.clone(), "xa,by.txt"));
        assert!(!matches(r, "a_and_b.txt"));
    }

    #[test]
    fn invalid_regex_is_reported_not_silently_dropped() {
        let mut r = rule(1, "([unclosed", "x");
        r.regex = true;
        let (compiled, errors) = compile_rules(&[r]);
        assert!(compiled.is_empty(), "a broken rule must not match anything");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].rule_id, 1);
        assert!(errors[0].message.starts_with("Contains:"));
    }

    #[test]
    fn disabled_rules_are_not_compiled() {
        let mut r = rule(1, "invoice", "x");
        r.enabled = false;
        let (compiled, errors) = compile_rules(&[r]);
        assert!(compiled.is_empty());
        assert!(errors.is_empty());
    }

    // --- folder tokens ------------------------------------------------------

    #[test]
    fn expands_name_stem_and_extension_tokens() {
        let p = Path::new("/src/Holiday_Clip.MP4");
        assert_eq!(expand_tokens("{ext}", p, None), "mp4");
        assert_eq!(expand_tokens("{stem}", p, None), "Holiday_Clip");
        assert_eq!(expand_tokens("{name}", p, None), "Holiday_Clip.MP4");
        assert_eq!(
            expand_tokens("media/{ext}", p, None),
            "media/mp4",
            "a slash in the template still nests"
        );
    }

    #[test]
    fn templates_without_tokens_pass_through() {
        assert_eq!(
            expand_tokens("Invoices", Path::new("/src/a.pdf"), None),
            "Invoices"
        );
    }

    #[test]
    fn expands_regex_captures() {
        let re = Regex::new(r"(\d{4})-(\d{2})_report").unwrap();
        let name = "2024-07_report.pdf";
        let caps = re.captures(name).unwrap();
        assert_eq!(
            expand_tokens("$1/$2", Path::new(name), Some(&caps)),
            "2024/07"
        );
    }

    #[test]
    fn unmatched_capture_groups_expand_to_nothing() {
        let re = Regex::new(r"report(?:_(v\d+))?").unwrap();
        let caps = re.captures("report.pdf").unwrap();
        assert_eq!(expand_tokens("out/$1", Path::new("report.pdf"), Some(&caps)), "out/");
    }

    #[test]
    fn token_values_cannot_inject_path_separators() {
        let re = Regex::new(r"(.+)\.txt").unwrap();
        let name = "a_b.txt";
        let caps = re.captures(name).unwrap();
        // A capture is user data; it must not be able to escape its segment.
        let expanded = expand_tokens("$1", Path::new(name), Some(&caps));
        assert!(!expanded.contains('/') && !expanded.contains('\\'));
    }

    #[test]
    fn date_tokens_use_the_file_modified_time() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("clip.mp4");
        std::fs::write(&path, b"x").unwrap();

        let expanded = expand_tokens("{YYYY}/{MM}", &path, None);
        let (year, month) = expanded.split_once('/').unwrap();
        assert_eq!(year.len(), 4, "got {expanded:?}");
        assert_eq!(month.len(), 2, "got {expanded:?}");
        assert!(year.parse::<i32>().unwrap() >= 2020);
        let m: u32 = month.parse().unwrap();
        assert!((1..=12).contains(&m));
    }

    #[test]
    fn date_tokens_fall_back_when_the_file_is_missing() {
        assert_eq!(
            expand_tokens("{YYYY}", Path::new("/definitely/not/here.txt"), None),
            "unknown"
        );
    }

    // --- folder name fallback ----------------------------------------------

    #[test]
    fn folder_name_strips_operators() {
        assert_eq!(sanitize_folder_name("invoice*2024"), "invoice 2024");
        assert_eq!(sanitize_folder_name("invoice,receipt"), "invoice receipt");
        assert_eq!(sanitize_folder_name("  spaced  "), "spaced");
    }
}
