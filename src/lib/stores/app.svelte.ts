let selectedPaths = $state<string[]>([]);
let sortStatus = $state<'idle' | 'sorting' | 'done' | 'error'>('idle');
let statusMessage = $state('');
let canUndo = $state(false);

export function getSelectedPaths(): string[] {
  return selectedPaths;
}

export function addPaths(paths: string[]) {
  const existing = new Set(selectedPaths);
  for (const p of paths) {
    if (!existing.has(p)) {
      selectedPaths.push(p);
    }
  }
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
