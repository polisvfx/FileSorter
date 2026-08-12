<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWebview } from '@tauri-apps/api/webview';
  import { open } from '@tauri-apps/plugin-dialog';
  import { onMount } from 'svelte';
  import { getSelectedPaths, addPaths, removePath, clearPaths, setPaths } from '$lib/stores/app.svelte';
  import { getRules } from '$lib/stores/rules.svelte';
  import { setSortStatus, getSortStatus, setStatusMessage, setCanUndo, getOutputDir, setOutputDir, getCopyMode, setCopyMode } from '$lib/stores/app.svelte';
  import {
    requestPreview,
    getPreviewEntries,
    getPreviewError,
    isPreviewPending,
    getMatchCountForPath
  } from '$lib/stores/preview.svelte';
  import type { SortResult } from '$lib/types';
  import { getFilename, getDirname, commonDirPrefix, hasSearchTerms } from '$lib/utils';

  let dragOver = $state(false);

  type SortMode = 'none' | 'name' | 'rules';
  let sortMode = $state<SortMode>('none');

  type ViewMode = 'files' | 'preview';
  let viewMode = $state<ViewMode>('files');

  // Reading each field explicitly is what registers the dependency, so editing a
  // rule's text re-runs the preview rather than only adding/removing rules.
  $effect(() => {
    const paths = getSelectedPaths().slice();
    const rules = getRules().map((r) => ({ ...r }));
    requestPreview(paths, rules, getOutputDir());
  });

  let sortedPaths = $derived.by(() => {
    const paths = getSelectedPaths();
    if (sortMode === 'none') return paths;
    return [...paths].sort((a, b) => {
      if (sortMode === 'name') {
        return getFilename(a).localeCompare(getFilename(b));
      }
      return getMatchCountForPath(b) - getMatchCountForPath(a);
    });
  });

  /** Group resolved destinations by target folder, relative to their common root. */
  let previewGroups = $derived.by(() => {
    const entries = getPreviewEntries();
    if (entries.length === 0) return [];

    // Root the labels at where the sort writes from, not at the destinations —
    // taking the common prefix of the destinations would swallow the folder name
    // whenever every file lands in the same one.
    const outputDir = getOutputDir();
    const root = outputDir
      ? outputDir.replace(/\\/g, '/').replace(/\/+$/, '')
      : commonDirPrefix(entries.map((e) => getDirname(e.original_path)));

    const byFolder = new Map<string, { name: string; from: string; renamed: boolean }[]>();

    for (const entry of entries) {
      const label = getDirname(entry.new_path).slice(root.length).replace(/^\//, '') || '.';
      const name = getFilename(entry.new_path);
      const from = getFilename(entry.original_path);
      const files = byFolder.get(label) ?? [];
      files.push({ name, from, renamed: name !== from });
      byFolder.set(label, files);
    }

    return [...byFolder.entries()]
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([folder, files]) => ({
        folder,
        files: files.sort((a, b) => a.name.localeCompare(b.name))
      }));
  });

  onMount(() => {
    // OS drag-and-drop only exists inside the Tauri shell. Guard it so opening the
    // frontend in a plain browser (`npm run dev` on its own) doesn't throw during
    // the effect flush, which takes the whole app down with it.
    let unlisten: Promise<() => void> | null = null;
    try {
      unlisten = getCurrentWebview().onDragDropEvent((event) => {
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
    } catch {
      return;
    }

    return () => {
      unlisten?.then((fn) => fn());
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
    // A Contains field of bare operators ("*", ",,") has nothing to search for.
    // Blank fields are just unfinished rules and are skipped quietly, but this
    // is a typo worth naming — it used to sweep up every file in the tree.
    const malformed = rules
      .map((r, i) => ({ number: i + 1, contains: r.contains.trim() }))
      .filter((r) => r.contains.length > 0 && !hasSearchTerms(r.contains));
    if (malformed.length > 0) {
      const list = malformed.map((r) => `#${r.number}`).join(', ');
      setStatusMessage(`Rule ${list}: "Contains" has no searchable text, only operators`);
      return;
    }

    const validRules = rules.filter((r) => hasSearchTerms(r.contains));
    if (validRules.length === 0) {
      setStatusMessage('Rules need the "Contains" field filled in');
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

      // Moved files live somewhere else now — keep the list pointing at them so a
      // second sort isn't a silent no-op against paths that no longer exist.
      if (!copyMode && result.operations.length > 0) {
        const moved = new Map(result.operations.map((op) => [op.original_path, op.new_path]));
        setPaths(paths.map((p) => moved.get(p) ?? p));
      }

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
      <div class="view-switch" role="group" aria-label="Panel view">
        <button class:active={viewMode === 'files'} onclick={() => (viewMode = 'files')}>Files</button>
        <button class:active={viewMode === 'preview'} onclick={() => (viewMode = 'preview')}>
          Preview
          {#if getPreviewEntries().length > 0}
            <span class="view-switch-count">{getPreviewEntries().length}</span>
          {/if}
        </button>
      </div>
      {#if viewMode === 'files'}
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
      {/if}
      {#if getSelectedPaths().length > 0}
        <button class="clear-btn" onclick={clearPaths} title="Clear all">
          Clear
        </button>
      {/if}
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
    {:else if viewMode === 'files'}
      <div class="path-list">
        {#each sortedPaths as path}
          <div class="path-item" class:has-rules={getMatchCountForPath(path) > 0}>
            <span class="path-text" title={path}>{shortenPath(path)}</span>
            {#if getMatchCountForPath(path) > 0}
              <span class="rule-badge" title="{getMatchCountForPath(path)} rule(s) match">{getMatchCountForPath(path)}</span>
            {/if}
            <button class="remove-path" onclick={() => removePath(path)} title="Remove">
              <svg width="12" height="12" viewBox="0 0 14 14" fill="none">
                <path d="M2 2L12 12M12 2L2 12" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
              </svg>
            </button>
          </div>
        {/each}
      </div>
    {:else if getPreviewError()}
      <div class="preview-empty">
        <p class="preview-error">Preview failed</p>
        <p class="hint">{getPreviewError()}</p>
      </div>
    {:else if previewGroups.length === 0}
      <div class="preview-empty">
        <p>{isPreviewPending() ? 'Working out destinations…' : 'No files match the current rules'}</p>
        <p class="hint">{isPreviewPending() ? '' : 'Nothing would be moved'}</p>
      </div>
    {:else}
      <div class="preview-list" class:stale={isPreviewPending()}>
        {#each previewGroups as group}
          <div class="preview-group">
            <div class="preview-folder">
              <svg width="12" height="12" viewBox="0 0 14 14" fill="none">
                <rect x="1" y="3" width="12" height="10" rx="1.5" stroke="currentColor" stroke-width="1.5"/>
                <path d="M1 5.5L1 3.5C1 2.67 1.67 2 2.5 2H5.5L7 4H11.5C12.33 4 13 4.67 13 5.5" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              <span class="preview-folder-name">{group.folder === '.' ? '(no subfolder)' : group.folder}</span>
              <span class="preview-folder-count">{group.files.length}</span>
            </div>
            {#each group.files as file}
              <div class="preview-file">
                <span class="preview-name" title={file.name}>{file.name}</span>
                {#if file.renamed}
                  <span class="preview-note" title="Renamed to avoid a name clash: {file.from}">renamed</span>
                {/if}
              </div>
            {/each}
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
    <button
      class="sort-btn"
      onclick={handleSort}
      disabled={getSelectedPaths().length === 0 || getRules().length === 0 || getSortStatus() === 'sorting'}
    >
      {getSortStatus() === 'sorting' ? 'Sorting…' : 'Sort Now'}
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

  .view-switch {
    display: flex;
    border: 1px solid var(--border);
    border-radius: 5px;
    overflow: hidden;
  }

  .view-switch button {
    display: flex;
    align-items: center;
    gap: 4px;
    background: none;
    border: none;
    padding: 3px 10px;
    font-size: 11px;
    font-family: inherit;
    color: var(--text-muted);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }

  .view-switch button:hover {
    color: var(--text);
  }

  .view-switch button.active {
    background: var(--accent);
    color: white;
  }

  .view-switch-count {
    font-size: 10px;
    font-weight: 600;
    opacity: 0.75;
  }

  .clear-btn {
    background: none;
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 3px 8px;
    font-size: 11px;
    color: var(--text-muted);
    cursor: pointer;
    transition: border-color 0.15s, color 0.15s;
  }

  .clear-btn:hover {
    border-color: var(--danger);
    color: var(--danger);
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

  .preview-empty {
    text-align: center;
    color: var(--text-muted);
    user-select: none;
    margin: auto;
  }

  .preview-empty p {
    margin: 2px 0;
    font-size: 14px;
  }

  .preview-empty .hint {
    font-size: 12px;
    opacity: 0.6;
  }

  .preview-error {
    color: var(--danger);
  }

  .preview-list {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 10px;
    transition: opacity 0.15s;
  }

  .preview-list.stale {
    opacity: 0.5;
  }

  .preview-group {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .preview-folder {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    color: var(--accent);
    font-size: 12px;
    font-weight: 600;
  }

  .preview-folder-name {
    font-family: 'SF Mono', 'Cascadia Code', 'Fira Code', monospace;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .preview-folder-count {
    font-size: 10px;
    font-weight: 600;
    color: var(--text-muted);
    background: var(--surface-3);
    border-radius: 8px;
    padding: 1px 6px;
    margin-left: auto;
    flex-shrink: 0;
  }

  .preview-file {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 10px 4px 26px;
    border-left: 1px solid var(--border);
    margin-left: 13px;
  }

  .preview-name {
    font-size: 12px;
    font-family: 'SF Mono', 'Cascadia Code', 'Fira Code', monospace;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .preview-note {
    font-size: 10px;
    font-weight: 600;
    color: var(--text-muted);
    border: 1px solid var(--border-hover);
    border-radius: 8px;
    padding: 0 6px;
    flex-shrink: 0;
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
