import type { Rule } from '$lib/types';

export function getFilename(path: string): string {
  const normalized = path.replace(/\\/g, '/');
  return normalized.substring(normalized.lastIndexOf('/') + 1).toLowerCase();
}

/// Evaluate a Contains/Contains-NOT expression against a (lowercased) filename.
/// "," is OR, "*" is AND, and AND binds tighter than OR: `a,b*c` means `a OR (b AND c)`.
function matchesExpr(filenameLower: string, expr: string): boolean {
  let anyGroup = false;
  for (const group of expr.split(',')) {
    const terms = group
      .split('*')
      .map((t) => t.trim().toLowerCase())
      .filter((t) => t.length > 0);
    if (terms.length === 0) continue;
    anyGroup = true;
    if (terms.every((t) => filenameLower.includes(t))) return true;
  }
  return !anyGroup;
}

export function ruleMatchesFile(rule: Rule, filename: string): boolean {
  const contains = rule.contains.trim();
  if (!contains) return false;
  if (!matchesExpr(filename, rule.contains)) return false;
  const containsNot = rule.contains_not?.trim();
  if (containsNot && matchesExpr(filename, containsNot)) return false;
  return true;
}
