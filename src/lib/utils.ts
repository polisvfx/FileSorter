/** Last path segment of a Windows or POSIX path. */
export function getFilename(path: string): string {
  const normalized = path.replace(/\\/g, '/');
  return normalized.substring(normalized.lastIndexOf('/') + 1);
}

/** Directory portion of a path, normalized to forward slashes. */
export function getDirname(path: string): string {
  const normalized = path.replace(/\\/g, '/');
  const lastSlash = normalized.lastIndexOf('/');
  return lastSlash >= 0 ? normalized.substring(0, lastSlash) : '';
}

/** Longest directory path shared by every input, compared segment by segment. */
export function commonDirPrefix(dirs: string[]): string {
  if (dirs.length === 0) return '';
  let parts = dirs[0].split('/');
  for (const dir of dirs.slice(1)) {
    const other = dir.split('/');
    let i = 0;
    while (i < parts.length && i < other.length && parts[i] === other[i]) i++;
    parts = parts.slice(0, i);
  }
  return parts.join('/');
}

/**
 * True when a Contains expression has at least one searchable term.
 *
 * Mirrors `parse_expr` in sort.rs: an expression of bare operators (`*`,
 * `,,`) has nothing to match on, so the UI refuses to sort with one rather than
 * letting it through as a no-op rule.
 */
export function hasSearchTerms(expr: string): boolean {
  return expr.split(',').some((group) => group.split('*').some((term) => term.trim().length > 0));
}
