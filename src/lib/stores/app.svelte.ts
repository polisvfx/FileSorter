import { invoke } from '@tauri-apps/api/core';
import type { PathInfo } from '$lib/types';

/**
 * The roots the user dropped or picked — files and folders as-is.
 *
 * Folders are deliberately *not* expanded into one entry per file: doing so put
 * tens of thousands of rows into the UI list and into the persisted session.
 * The Rust side walks them when it actually needs the files.
 */
let roots = $state<PathInfo[]>([]);
let sortStatus = $state<'idle' | 'sorting' | 'done' | 'error'>('idle');
let statusMessage = $state('');
let canUndo = $state(false);
let outputDir = $state<string | null>(null);
let copyMode = $state(false);
let progress = $state<{ processed: number; total: number; current: string } | null>(null);

export function getRoots(): PathInfo[] {
  return roots;
}

export function getRootPaths(): string[] {
  return roots.map((r) => r.path);
}

/** Total files across every root — what a sort would actually walk. */
export function getTotalFileCount(): number {
  return roots.reduce((sum, r) => sum + r.file_count, 0);
}

export async function addPaths(paths: string[]) {
  const existing = new Set(roots.map((r) => r.path));
  const fresh = paths.filter((p) => !existing.has(p));
  if (fresh.length === 0) return;

  const infos = await invoke<PathInfo[]>('inspect_paths', { paths: fresh });
  roots = [...roots, ...infos];
}

/** Re-describe the current roots, optionally remapping paths that moved. */
export async function refreshRoots(remap?: Map<string, string>) {
  if (roots.length === 0) return;
  const paths = roots.map((r) => remap?.get(r.path) ?? r.path);
  roots = await invoke<PathInfo[]>('inspect_paths', { paths });
}

/** Replace the roots wholesale (session restore). Drops anything that's gone. */
export async function setPaths(paths: string[]) {
  roots = paths.length > 0 ? await invoke<PathInfo[]>('inspect_paths', { paths }) : [];
}

export function removePath(path: string) {
  roots = roots.filter((r) => r.path !== path);
}

export function clearPaths() {
  roots = [];
}

export function getSortStatus() {
  return sortStatus;
}

export function setSortStatus(status: 'idle' | 'sorting' | 'done' | 'error') {
  sortStatus = status;
}

export function getStatusMessage() {
  return statusMessage;
}

export function setStatusMessage(msg: string) {
  statusMessage = msg;
}

export function getCanUndo() {
  return canUndo;
}

export function setCanUndo(value: boolean) {
  canUndo = value;
}

export function getOutputDir(): string | null {
  return outputDir;
}

export function setOutputDir(dir: string | null) {
  outputDir = dir;
}

export function getCopyMode(): boolean {
  return copyMode;
}

export function setCopyMode(value: boolean) {
  copyMode = value;
}

export function getProgress() {
  return progress;
}

export function setProgress(value: { processed: number; total: number; current: string } | null) {
  progress = value;
}
