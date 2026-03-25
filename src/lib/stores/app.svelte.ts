import { invoke } from '@tauri-apps/api/core';

let selectedPaths = $state<string[]>([]);
let sortStatus = $state<'idle' | 'sorting' | 'done' | 'error'>('idle');
let statusMessage = $state('');
let canUndo = $state(false);
let outputDir = $state<string | null>(null);
let copyMode = $state(false);

export function getSelectedPaths(): string[] {
  return selectedPaths;
}

export async function addPaths(paths: string[]) {
  const resolved = await invoke<string[]>('resolve_paths', { paths });
  const existing = new Set(selectedPaths);
  for (const p of resolved) {
    if (!existing.has(p)) {
      selectedPaths.push(p);
    }
  }
}

export function setPaths(paths: string[]) {
  selectedPaths = paths;
}

export function removePath(path: string) {
  selectedPaths = selectedPaths.filter((p) => p !== path);
}

export function clearPaths() {
  selectedPaths = [];
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
