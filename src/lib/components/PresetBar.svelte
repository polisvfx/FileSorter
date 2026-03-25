<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { getRules, setRules } from '$lib/stores/rules.svelte';
  import { setStatusMessage } from '$lib/stores/app.svelte';
  import type { Rule } from '$lib/types';

  let presets = $state<string[]>([]);
  let selectedPreset = $state('');
  let showSaveInput = $state(false);
  let newPresetName = $state('');

  async function loadPresetList() {
    try {
      presets = await invoke<string[]>('list_presets');
    } catch {
      presets = [];
    }
  }

  async function handleLoad() {
    if (!selectedPreset) return;
    try {
      const rules = await invoke<Rule[]>('load_preset', { name: selectedPreset });
      setRules(rules);
      setStatusMessage(`Loaded preset "${selectedPreset}"`);
    } catch (err) {
      setStatusMessage(`Failed to load preset: ${err}`);
    }
  }

  async function handleSave() {
    const name = newPresetName.trim();
    if (!name) return;
    try {
      await invoke('save_preset', { name, rules: getRules() });
      showSaveInput = false;
      newPresetName = '';
      setStatusMessage(`Saved preset "${name}"`);
      await loadPresetList();
      selectedPreset = name;
    } catch (err) {
      setStatusMessage(`Failed to save preset: ${err}`);
    }
  }

  async function handleDelete() {
    if (!selectedPreset) return;
    try {
      await invoke('delete_preset', { name: selectedPreset });
      setStatusMessage(`Deleted preset "${selectedPreset}"`);
      selectedPreset = '';
      await loadPresetList();
    } catch (err) {
      setStatusMessage(`Failed to delete preset: ${err}`);
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') handleSave();
    if (e.key === 'Escape') {
      showSaveInput = false;
      newPresetName = '';
    }
  }

  // Load presets on mount
  $effect(() => {
    loadPresetList();
  });
</script>

<div class="preset-bar">
  <span class="preset-label">Presets</span>

  <select bind:value={selectedPreset} class="preset-select">
    <option value="">Select preset...</option>
    {#each presets as preset}
      <option value={preset}>{preset}</option>
    {/each}
  </select>

  <button class="preset-btn" onclick={handleLoad} disabled={!selectedPreset} title="Load preset">
    Load
  </button>

  {#if showSaveInput}
    <input
      type="text"
      class="save-input"
      placeholder="Preset name"
      bind:value={newPresetName}
      onkeydown={handleKeydown}
    />
    <button class="preset-btn accent" onclick={handleSave} disabled={!newPresetName.trim()}>
      OK
    </button>
    <button class="preset-btn" onclick={() => { showSaveInput = false; newPresetName = ''; }}>
      Cancel
    </button>
  {:else}
    <button class="preset-btn" onclick={() => { showSaveInput = true; }} title="Save current rules as preset">
      Save
    </button>
  {/if}

  {#if selectedPreset}
    <button class="preset-btn danger" onclick={handleDelete} title="Delete preset">
      Del
    </button>
  {/if}
</div>

<style>
  .preset-bar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 16px;
    background: var(--surface-1);
    border-bottom: 1px solid var(--border);
  }

  .preset-label {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-right: 4px;
  }

  .preset-select {
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 5px 8px;
    font-size: 12px;
    color: var(--text);
    min-width: 150px;
    outline: none;
  }

  .preset-select:focus {
    border-color: var(--accent);
  }

  .save-input {
    background: var(--surface-2);
    border: 1px solid var(--accent);
    border-radius: 6px;
    padding: 5px 8px;
    font-size: 12px;
    color: var(--text);
    width: 140px;
    outline: none;
  }

  .preset-btn {
    padding: 5px 12px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s;
  }

  .preset-btn:hover:not(:disabled) {
    background: var(--surface-3);
    border-color: var(--border-hover);
  }

  .preset-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .preset-btn.accent {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }

  .preset-btn.accent:hover:not(:disabled) {
    background: var(--accent-hover);
  }

  .preset-btn.danger:hover:not(:disabled) {
    color: var(--danger);
    border-color: var(--danger);
  }
</style>
