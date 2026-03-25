<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import {
    getSortStatus,
    getStatusMessage,
    getCanUndo,
    setCanUndo,
    setStatusMessage,
    setSortStatus
  } from '$lib/stores/app.svelte';

  async function handleUndo() {
    setStatusMessage('Undoing...');
    try {
      const errors = await invoke<string[]>('undo_last_sort');
      if (errors.length > 0) {
        setStatusMessage(`Undo completed with ${errors.length} error(s)`);
      } else {
        setStatusMessage('Undo completed successfully');
      }
      setSortStatus('idle');
      setCanUndo(false);
    } catch (err) {
      setStatusMessage(`Undo failed: ${err}`);
    }
  }
</script>

<div class="status-bar">
  <div class="status-left">
    {#if getSortStatus() === 'sorting'}
      <div class="spinner"></div>
    {:else if getSortStatus() === 'done'}
      <span class="status-icon done">&#10003;</span>
    {:else if getSortStatus() === 'error'}
      <span class="status-icon error">!</span>
    {/if}
    <span class="status-text">{getStatusMessage() || 'Ready'}</span>
  </div>

  {#if getCanUndo()}
    <button class="undo-btn" onclick={handleUndo}>
      <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
        <path d="M3 5L1 3L3 1" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
        <path d="M1 3H9C11.2 3 13 4.8 13 7C13 9.2 11.2 11 9 11H5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
      </svg>
      Undo
    </button>
  {/if}
</div>

<style>
  .status-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 16px;
    background: var(--surface-1);
    border-top: 1px solid var(--border);
  }

  .status-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .status-text {
    font-size: 12px;
    color: var(--text-muted);
  }

  .status-icon {
    font-size: 12px;
    font-weight: 700;
    width: 18px;
    height: 18px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
  }

  .status-icon.done {
    color: var(--success);
    background: rgba(52, 199, 89, 0.15);
  }

  .status-icon.error {
    color: var(--danger);
    background: rgba(255, 59, 48, 0.15);
  }

  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .undo-btn {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 5px 14px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s;
  }

  .undo-btn:hover {
    background: var(--surface-3);
    border-color: var(--accent);
    color: var(--accent);
  }
</style>
