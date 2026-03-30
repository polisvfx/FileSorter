import type { Rule } from '$lib/types';

export function getFilename(path: string): string {
  const normalized = path.replace(/\\/g, '/');
  return normalized.substring(normalized.lastIndexOf('/') + 1).toLowerCase();
}

export function ruleMatchesFile(rule: Rule, filename: string): boolean {
  const contains = rule.contains.trim().toLowerCase();
  if (!contains) return false;
  if (!filename.includes(contains)) return false;
  const containsNot = rule.contains_not?.trim().toLowerCase();
  if (containsNot && filename.includes(containsNot)) return false;
  return true;
}
