import type { Rule } from '$lib/types';

const STORAGE_KEY = 'filesorter-session';

interface SessionState {
  rules: Rule[];
  outputDir: string | null;
  copyMode: boolean;
  selectedPaths: string[];
}

export function saveSession(state: SessionState): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch {
    // silently ignore quota errors
  }
}

export function loadSession(): SessionState | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    return JSON.parse(raw) as SessionState;
  } catch {
    return null;
  }
}
