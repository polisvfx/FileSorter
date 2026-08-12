import { invoke } from '@tauri-apps/api/core';
import type { PreviewEntry, PreviewResult, Rule, RuleError } from '$lib/types';

const DEBOUNCE_MS = 200;

let entries = $state<PreviewEntry[]>([]);
let ruleErrors = $state<RuleError[]>([]);
let error = $state<string | null>(null);
let pending = $state(false);

let timer: ReturnType<typeof setTimeout> | null = null;
/** Guards against an older in-flight request overwriting a newer result. */
let runToken = 0;

// Counts are memoised against the entries array's identity rather than held in a
// module-level $derived: `entries` is only ever reassigned, never mutated, so an
// identity check is enough, and reading it here registers the dependency in
// whichever component effect calls through.
let countedEntries: PreviewEntry[] | null = null;
let matchCountByRule = new Map<number, number>();
let normalizedOriginals: string[] = [];

const normalize = (p: string) => p.replace(/\\/g, '/');

function ensureCounts() {
  const current = entries;
  if (countedEntries === current) return;
  countedEntries = current;

  matchCountByRule = new Map();
  for (const entry of current) {
    for (const id of entry.matched_rule_ids) {
      matchCountByRule.set(id, (matchCountByRule.get(id) ?? 0) + 1);
    }
  }
  normalizedOriginals = current.map((e) => normalize(e.original_path));
}

export function getPreviewEntries(): PreviewEntry[] {
  return entries;
}

export function getPreviewError(): string | null {
  return error;
}

export function isPreviewPending(): boolean {
  return pending;
}

/**
 * How many files under this root would move. For a dropped folder that's the
 * count across its whole tree; for a single file it's 0 or 1.
 */
export function getMatchCountUnder(root: string): number {
  ensureCounts();
  const prefix = normalize(root);
  const nested = prefix.endsWith('/') ? prefix : prefix + '/';
  let count = 0;
  for (const original of normalizedOriginals) {
    if (original === prefix || original.startsWith(nested)) count++;
  }
  return count;
}

/** How many files this rule would actually claim during a real sort. */
export function getMatchCountForRule(ruleId: number): number {
  ensureCounts();
  return matchCountByRule.get(ruleId) ?? 0;
}

/** Compile error for this rule, if its pattern is currently invalid. */
export function getRuleError(ruleId: number): string | null {
  return ruleErrors.find((e) => e.rule_id === ruleId)?.message ?? null;
}

export function hasRuleErrors(): boolean {
  return ruleErrors.length > 0;
}

function reset() {
  entries = [];
  ruleErrors = [];
  error = null;
  pending = false;
}

/**
 * Recompute the preview, debounced so typing in a rule field doesn't fire a
 * directory walk per keystroke.
 */
export function requestPreview(paths: string[], rules: Rule[], outputDir: string | null) {
  if (timer) clearTimeout(timer);

  if (paths.length === 0 || rules.length === 0) {
    runToken++;
    reset();
    return;
  }

  pending = true;
  timer = setTimeout(async () => {
    const token = ++runToken;
    try {
      const result = await invoke<PreviewResult>('preview_sort', { paths, rules, outputDir });
      if (token !== runToken) return;
      entries = result.entries;
      ruleErrors = result.rule_errors;
      error = null;
    } catch (err) {
      if (token !== runToken) return;
      entries = [];
      ruleErrors = [];
      error = String(err);
    } finally {
      if (token === runToken) pending = false;
    }
  }, DEBOUNCE_MS);
}
