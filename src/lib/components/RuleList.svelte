<script lang="ts">
  import { dndzone } from 'svelte-dnd-action';
  import type { Rule } from '$lib/types';
  import { getRules, addRule, reorderRules } from '$lib/stores/rules.svelte';
  import RuleRow from './RuleRow.svelte';

  const flipDurationMs = 200;

  function handleDndConsider(e: CustomEvent<{ items: Rule[] }>) {
    reorderRules(e.detail.items);
  }

  function handleDndFinalize(e: CustomEvent<{ items: Rule[] }>) {
    reorderRules(e.detail.items);
  }
</script>

<div class="rule-list">
  <div class="rule-list-header">
    <h2>Sorting Rules</h2>
    <span class="rule-count">{getRules().length} rules</span>
  </div>

  <div
    class="rules-container"
    use:dndzone={{ items: getRules(), flipDurationMs, type: 'rules' }}
    onconsider={handleDndConsider}
    onfinalize={handleDndFinalize}
  >
    {#each getRules() as rule, i (rule.id)}
      <RuleRow {rule} index={i} />
    {/each}
  </div>

  <button class="add-rule-btn" onclick={addRule}>
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <path d="M7 1V13M1 7H13" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
    </svg>
    Add Rule
  </button>
</div>

<style>
  .rule-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
    height: 100%;
  }

  .rule-list-header {
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

  .rule-count {
    font-size: 12px;
    color: var(--text-muted);
  }

  .rules-container {
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex: 1;
    overflow-y: auto;
    min-height: 60px;
    padding: 2px;
  }

  .add-rule-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 10px;
    background: var(--surface-2);
    border: 1px dashed var(--border);
    border-radius: 8px;
    color: var(--text-muted);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: color 0.15s, border-color 0.15s, background 0.15s;
  }

  .add-rule-btn:hover {
    color: var(--accent);
    border-color: var(--accent);
    background: var(--surface-3);
  }
</style>
