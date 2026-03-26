<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWebview } from '@tauri-apps/api/webview';
  import { open } from '@tauri-apps/plugin-dialog';
  import { onMount } from 'svelte';
  import { getSelectedPaths, addPaths, removePath } from '$lib/stores/app.svelte';
  import { getRules } from '$lib/stores/rules.svelte';
  import { setSortStatus, setStatusMessage, setCanUndo, getOutputDir, setOutputDir, getCopyMode, setCopyMode } from '$lib/stores/app.svelte';
  import type { Rule, SortResult } from '$lib/types';

  let dragOver = $state(false);

  type SortMode = 'none' | 'name' | 'rules';
  let sortMode = $state<SortMode>('none');

  function getFilename(path: string): string {
    const normalized = path.replace(/\\/g, '/');
    return normalized.substring(normalized.lastIndexOf('/') + 1).toLowerCase();
  }

  function ruleMatchesFile(rule: Rule, filename: string): boolean {
    const contains = rule.contains.trim().toLowerCase();
    if (!contains || !rule.target_folder.trim()) return false;
    if (!filename.includes(contains)) return false;
    const containsNot = rule.contains_not?.trim().toLowerCase();
    if (containsNot && filename.includes(containsNot)) return false;
    return true;
  }

  let matchCounts = $derived(
    new Map(
      getSelectedPaths().map((path) => {
        const filename = getFilename(path);
        const count = getRules().filter((r) => ruleMatchesFile(r, filename)).length;
        return [path, count] as const;
      })
    )
  );

  let sortedPaths = $derived.by(() => {
    const paths = getSelectedPaths();
    if (sortMode === 'none') return paths;
    return [...paths].sort((a, b) => {
      if (sortMode === 'name') {
        return getFilename(a).localeCompare(getFilename(b));
      }
      return (matchCounts.get(b) ?? 0) - (matchCounts.get(a) ?? 0);
    });
  });

  onMount(() => {
    const unlisten = getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === 'over') {
        dragOver = true;
      } else if (event.payload.type === 'leave') {
        dragOver = false;
      } else if (event.payload.type === 'drop') {
        dragOver = false;
        if (event.payload.paths.length > 0) {
          addPaths(event.payload.paths);
        }
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  });

  async function handleBrowse() {
    try {
      const selected = await open({
        multiple: true,
        directory: false,
        title: 'Select files to sort'
      });
      if (selected) {
        const paths = Array.isArray(selected) ? selected : [selected];
        await addPaths(paths);
      }
    } catch {
      // user cancelled
    }
  }

  async function handleBrowseFolder() {
    try {
      const selected = await open({
        multiple: false,
        directory: true,
        title: 'Select folder to sort'
      });
      if (selected) {
        await addPaths([selected]);
      }
    } catch {
      // user cancelled
    }
  }

  function handleDragOver(e: DragEvent) {
    e.preventDefault();
  }

  function handleDragLeave(e: DragEvent) {
    e.preventDefault();
  }

  function handleDrop(e: DragEvent) {
    e.preventDefault();
  }

  async function handleBrowseOutput() {
    try {
      const selected = await open({
        multiple: false,
        directory: true,
        title: 'Select output directory'
      });
      if (selected) {
        setOutputDir(selected);
      }
    } catch {
      // user cancelled
    }
  }

  function getUniqueParentDirs(paths: string[]): Set<string> {
    return new Set(
      paths.map((p) => {
        const normalized = p.replace(/\\/g, '/');
        const lastSlash = normalized.lastIndexOf('/');
        return lastSlash >= 0 ? normalized.substring(0, lastSlash) : normalized;
      })
    );
  }

  async function handleSort() {
    const paths = getSelectedPaths();
    const rules = getRules();
    const copyMode = getCopyMode();

    if (paths.length === 0) {
      setStatusMessage('No files or folders selected');
      return;
    }
    if (rules.length === 0) {
      setStatusMessage('No rules defined');
      return;
    }
    const validRules = rules.filter((r) => r.contains.trim() && r.target_folder.trim());
    if (validRules.length === 0) {
      setStatusMessage('Rules need both "Contains" and "Folder" fields');
      return;
    }

    const parentDirs = getUniqueParentDirs(paths);
    if (parentDirs.size > 1 && !getOutputDir()) {
      setStatusMessage('Output directory required when files come from multiple folders');
      return;
    }

    setSortStatus('sorting');
    setStatusMessage(copyMode ? 'Copying files...' : 'Sorting files...');

    const verb = copyMode ? 'Copied' : 'Moved';

    try {
      const result = await invoke<SortResult>('sort_files', {
        paths,
        rules: validRules,
        outputDir: getOutputDir(),
        copyMode
      });

      if (result.errors.length > 0) {
        setSortStatus('error');
        setStatusMessage(`Done with ${result.errors.length} error(s). ${verb} ${result.operations.length} file(s).`);
      } else {
        setSortStatus('done');
        setStatusMessage(`${verb} ${result.operations.length} file(s) successfully.`);
      }
      setCanUndo(result.operations.length > 0);
    } catch (err) {
      setSortStatus('error');
      setStatusMessage(`Error: ${err}`);
    }
  }

  function shortenPath(p: string): string {
    const parts = p.replace(/\\/g, '/').split('/');
    if (parts.length <= 3) return parts.join('/');
    return `.../${parts.slice(-2).join('/')}`;
  }
</script>

<div class="drop-zone-panel">
  <div class="drop-zone-header">
    <h2>Files & Folders</h2>
    <div class="header-controls">
      <button class="sort-toggle" onclick={() => {
        if (sortMode === 'none') sortMode = 'name';
        else if (sortMode === 'name') sortMode = 'rules';
        else sortMode = 'none';
      }}>
        <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
          <path d="M2 3H10M3 6H9M4 9H8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
        {sortMode === 'none' ? 'Unsorted' : sortMode === 'name' ? 'By Name' : 'By Rules'}
      </button>
      <span class="path-count">{getSelectedPaths().length} selected</span>
    </div>
  </div>

  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="drop-area"
    class:drag-over={dragOver}
    class:has-items={getSelectedPaths().length > 0}
    ondragover={handleDragOver}
    ondragleave={handleDragLeave}
    ondrop={handleDrop}
    onclick={getSelectedPaths().length === 0 ? handleBrowse : undefined}
  >
    {#if getSelectedPaths().length === 0}
      <div class="drop-placeholder">
        <svg width="40" height="40" viewBox="0 0 40 40" fill="none">
          <rect x="4" y="8" width="32" height="26" rx="3" stroke="currentColor" stroke-width="2"/>
          <path d="M4 14H36" stroke="currentColor" stroke-width="2"/>
          <path d="M4 14L8 8H18L22 14" stroke="currentColor" stroke-width="2"/>
        </svg>
        <p>Drop files or folders here</p>
        <p class="hint">or click to browse</p>
      </div>
    {:else}
      <div class="path-list">
        {#each sortedPaths as path}
          <div class="path-item" class:has-rules={(matchCounts.get(path) ?? 0) > 0}>
            <span class="path-text" title={path}>{shortenPath(path)}</span>
            {#if (matchCounts.get(path) ?? 0) > 0}
              <span class="rule-badge" title="{matchCounts.get(path)} rule(s) match">{matchCounts.get(path)}</span>
            {/if}
            <button class="remove-path" onclick={() => removePath(path)} title="Remove">
              <svg width="12" height="12" viewBox="0 0 14 14" fill="none">
                <path d="M2 2L12 12M12 2L2 12" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
              </svg>
            </button>
          </div>
        {/each}
      </div>
    {/if}
  </div>

  {#if getOutputDir()}
    <div class="output-dir-bar">
      <span class="output-label">Output:</span>
      <span class="output-path" title={getOutputDir()}>{shortenPath(getOutputDir()!)}</span>
      <button class="remove-path" onclick={() => setOutputDir(null)} title="Clear output directory">
        <svg width="12" height="12" viewBox="0 0 14 14" fill="none">
          <path d="M2 2L12 12M12 2L2 12" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        </svg>
      </button>
    </div>
  {/if}

  <div class="drop-zone-actions">
    <div class="browse-buttons">
      <button class="browse-btn" onclick={handleBrowse}>
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
          <path d="M1 7H13M7 1V13" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        </svg>
        Files
      </button>
      <button class="browse-btn" onclick={handleBrowseFolder}>
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
          <rect x="1" y="3" width="12" height="10" rx="1.5" stroke="currentColor" stroke-width="1.5"/>
          <path d="M1 5.5L1 3.5C1 2.67 1.67 2 2.5 2H5.5L7 4H11.5C12.33 4 13 4.67 13 5.5" stroke="currentColor" stroke-width="1.5"/>
        </svg>
        Folder
      </button>
      <button class="browse-btn" onclick={handleBrowseOutput} title="Set output directory">
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
          <path d="M7 1V10M7 10L4 7M7 10L10 7" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          <path d="M1 11V12.5C1 12.78 1.22 13 1.5 13H12.5C12.78 13 13 12.78 13 12.5V11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
        Output
      </button>
    </div>
    <label class="copy-toggle" title="Copy files instead of moving them">
      <input type="checkbox" checked={getCopyMode()} onchange={(e) => setCopyMode(e.currentTarget.checked)} />
      <span>Copy</span>
    </label>
    <button class="sort-btn" onclick={handleSort} disabled={getSelectedPaths().length === 0 || getRules().length === 0}>
      Sort Now
    </button>
  </div>
</div>

<style>
  .drop-zone-panel {
    display: flex;
    flex-direction: column;
    gap: 10px;
    height: 100%;
  }

  .drop-zone-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-bottom: 4px;
  }

  h2 {
    font-size: 14px;
    font-weight: 700;
    color: var(--text);
    margin: 0;
  }

  .header-controls {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .sort-toggle {
    display: flex;
    align-items: center;
    gap: 4px;
    background: none;
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 3px 8px;
    font-size: 11px;
    color: var(--text-muted);
    cursor: pointer;
    transition: border-color 0.15s, color 0.15s;
  }

  .sort-toggle:hover {
    border-color: var(--border-hover);
    color: var(--text);
  }

  .path-count {
    font-size: 12px;
    color: var(--text-muted);
  }

  .drop-area {
    flex: 1;
    border: 2px dashed var(--border);
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: border-color 0.2s, background 0.2s;
    overflow-y: auto;
    min-height: 120px;
  }

  .drop-area:hover {
    border-color: var(--accent);
    background: var(--surface-2);
  }

  .drop-area.drag-over {
    border-color: var(--accent);
    background: rgba(102, 153, 204, 0.08);
  }

  .drop-area.has-items {
    cursor: default;
    align-items: flex-start;
    padding: 8px;
  }

  .drop-placeholder {
    text-align: center;
    color: var(--text-muted);
    user-select: none;
  }

  .drop-placeholder svg {
    opacity: 0.4;
    margin-bottom: 8px;
  }

  .drop-placeholder p {
    margin: 2px 0;
    font-size: 14px;
  }

  .drop-placeholder .hint {
    font-size: 12px;
    opacity: 0.6;
  }

  .path-list {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .path-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 10px;
    background: var(--surface-2);
    border-radius: 6px;
    border: 1px solid var(--border);
    transition: border-color 0.15s, background 0.15s;
  }

  .path-item.has-rules {
    border-left: 3px solid var(--accent);
    background: rgba(102, 153, 204, 0.06);
  }

  .path-text {
    font-size: 12px;
    font-family: 'SF Mono', 'Cascadia Code', 'Fira Code', monospace;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }

  .rule-badge {
    font-size: 10px;
    font-weight: 600;
    background: var(--accent);
    color: white;
    border-radius: 8px;
    padding: 1px 6px;
    min-width: 16px;
    text-align: center;
    flex-shrink: 0;
    margin-left: 6px;
  }

  .remove-path {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 2px;
    border-radius: 3px;
    display: flex;
    align-items: center;
    transition: color 0.15s;
    flex-shrink: 0;
    margin-left: 8px;
  }

  .remove-path:hover {
    color: var(--danger);
  }

  .drop-zone-actions {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .browse-buttons {
    display: flex;
    gap: 6px;
  }

  .browse-btn {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 8px 14px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 7px;
    color: var(--text);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s;
  }

  .browse-btn:hover {
    background: var(--surface-3);
    border-color: var(--border-hover);
  }

  .output-dir-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 6px;
  }

  .output-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    flex-shrink: 0;
  }

  .output-path {
    font-size: 12px;
    font-family: 'SF Mono', 'Cascadia Code', 'Fira Code', monospace;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }

  .copy-toggle {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    color: var(--text);
    cursor: pointer;
    user-select: none;
  }

  .copy-toggle input[type="checkbox"] {
    accent-color: var(--accent);
    cursor: pointer;
  }

  .sort-btn {
    margin-left: auto;
    padding: 8px 24px;
    background: var(--accent);
    border: none;
    border-radius: 7px;
    color: white;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s, transform 0.1s;
  }

  .sort-btn:hover:not(:disabled) {
    background: var(--accent-hover);
  }

  .sort-btn:active:not(:disabled) {
    transform: scale(0.97);
  }

  .sort-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>
