import { invoke } from '@tauri-apps/api/core';
import type { PreviewEntry, Rule } from '$lib/types';

const DEBOUNCE_MS = 200;

let entries = $state<PreviewEntry[]>([]);
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
let matchCountByPath = new Map<string, number>();
let matchCountByRule = new Map<number, number>();

function ensureCounts() {
  const current = entries;
  if (countedEntries === current) return;
  countedEntries = current;

  matchCountByPath = new Map(current.map((e) => [e.original_path, e.matched_rule_ids.length]));
  matchCountByRule = new Map();
  for (const entry of current) {
    for (const id of entry.matched_rule_ids) {
      matchCountByRule.set(id, (matchCountByRule.get(id) ?? 0) + 1);
    }
  }
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

/** How many rules would act on this file, honouring enabled and stop-on-match. */
export function getMatchCountForPath(path: string): number {
  ensureCounts();
  return matchCountByPath.get(path) ?? 0;
}

/** How many files this rule would actually claim during a real sort. */
export function getMatchCountForRule(ruleId: number): number {
  ensureCounts();
  return matchCountByRule.get(ruleId) ?? 0;
}

function reset() {
  entries = [];
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
      const result = await invoke<PreviewEntry[]>('preview_sort', { paths, rules, outputDir });
      if (token !== runToken) return;
      entries = result;
      error = null;
    } catch (err) {
      if (token !== runToken) return;
      entries = [];
      error = String(err);
    } finally {
      if (token === runToken) pending = false;
    }
  }, DEBOUNCE_MS);
}
