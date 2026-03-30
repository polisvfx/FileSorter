<script lang="ts">
  import type { Rule } from '$lib/types';
  import { updateRule, removeRule } from '$lib/stores/rules.svelte';
  import { getSelectedPaths } from '$lib/stores/app.svelte';
  import { getFilename, ruleMatchesFile } from '$lib/utils';

  let { rule, index }: { rule: Rule; index: number } = $props();

  let matchCount = $derived(
    getSelectedPaths().filter((p) => ruleMatchesFile(rule, getFilename(p))).length
  );
</script>

<div class="rule-row">
  <span class="rule-number">{index + 1}</span>

  <div class="rule-fields">
    <div class="field">
      <label for="contains-{rule.id}">Contains</label>
      <input
        id="contains-{rule.id}"
        type="text"
        value={rule.contains}
        placeholder="e.g. invoice"
        oninput={(e) => updateRule(rule.id, 'contains', (e.target as HTMLInputElement).value)}
      />
    </div>

    <div class="field">
      <label for="not-{rule.id}">Not</label>
      <input
        id="not-{rule.id}"
        type="text"
        value={rule.contains_not ?? ''}
        placeholder="(optional)"
        oninput={(e) => {
          const val = (e.target as HTMLInputElement).value;
          updateRule(rule.id, 'contains_not', val || null);
        }}
      />
    </div>

    <div class="field">
      <label for="target-{rule.id}">Folder</label>
      <input
        id="target-{rule.id}"
        type="text"
        value={rule.target_folder}
        placeholder={rule.contains.trim() || 'e.g. Invoices'}
        oninput={(e) => updateRule(rule.id, 'target_folder', (e.target as HTMLInputElement).value)}
      />
    </div>
  </div>

  {#if matchCount > 0}
    <span class="match-badge" title="{matchCount} file{matchCount === 1 ? '' : 's'} match">{matchCount}</span>
  {/if}

  <button class="delete-btn" onclick={() => removeRule(rule.id)} title="Remove rule">
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <path d="M2 2L12 12M12 2L2 12" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
    </svg>
  </button>
</div>

<style>
  .rule-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    background: var(--surface-2);
    border-radius: 8px;
    border: 1px solid var(--border);
    transition: border-color 0.15s, box-shadow 0.15s;
  }

  .rule-row:hover {
    border-color: var(--border-hover);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
  }

  .rule-number {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-muted);
    min-width: 20px;
    text-align: center;
    user-select: none;
  }

  .rule-fields {
    display: flex;
    gap: 8px;
    flex: 1;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
  }

  .field:nth-child(2) {
    flex: 0.7;
  }

  label {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
    font-weight: 600;
  }

  input {
    background: var(--surface-1);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 6px 8px;
    font-size: 13px;
    color: var(--text);
    outline: none;
    transition: border-color 0.15s;
  }

  input:focus {
    border-color: var(--accent);
  }

  input::placeholder {
    color: var(--text-muted);
    opacity: 0.5;
  }

  .match-badge {
    font-size: 11px;
    font-weight: 600;
    color: white;
    background: var(--accent);
    border-radius: 10px;
    padding: 1px 7px;
    min-width: 20px;
    text-align: center;
    flex-shrink: 0;
  }

  .delete-btn {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 4px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: color 0.15s, background 0.15s;
  }

  .delete-btn:hover {
    color: var(--danger);
    background: rgba(255, 59, 48, 0.1);
  }
</style>
