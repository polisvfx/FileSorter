<script lang="ts">
  import type { Rule } from '$lib/types';
  import type { MatchScope } from '$lib/types';
  import { updateRule, removeRule, duplicateRule } from '$lib/stores/rules.svelte';
  import { getMatchCountForRule, getRuleError } from '$lib/stores/preview.svelte';

  let { rule, index }: { rule: Rule; index: number } = $props();

  // Counted from the preview, so this reflects what the rule actually claims —
  // disabled rules and files taken by an earlier stop-on-match rule don't count.
  let matchCount = $derived(getMatchCountForRule(rule.id));
  let ruleError = $derived(getRuleError(rule.id));

  const SCOPES: { value: MatchScope; label: string; title: string }[] = [
    { value: 'name', label: 'Name', title: 'Match the filename including its extension' },
    { value: 'stem', label: 'Stem', title: 'Match the filename without its extension' },
    { value: 'extension', label: 'Ext', title: 'Match only the extension, without the dot' },
    { value: 'path', label: 'Path', title: 'Match the full path, including parent folders' }
  ];
</script>

<div class="rule-row" class:disabled={!rule.enabled}>
  <span class="rule-number">{index + 1}</span>

  <label class="toggle-switch" title={rule.enabled ? 'Disable rule' : 'Enable rule'}>
    <input
      type="checkbox"
      checked={rule.enabled}
      onchange={(e) => updateRule(rule.id, 'enabled', (e.target as HTMLInputElement).checked)}
    />
    <span class="toggle-slider"></span>
  </label>

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
        title="Supports tokens: {'{ext}'} {'{stem}'} {'{name}'} {'{YYYY}'} {'{MM}'} {'{DD}'}, and $1..$9 for regex captures"
        oninput={(e) => updateRule(rule.id, 'target_folder', (e.target as HTMLInputElement).value)}
      />
    </div>

    <div class="field field-scope">
      <label for="scope-{rule.id}">Match</label>
      <select
        id="scope-{rule.id}"
        value={rule.scope}
        onchange={(e) => updateRule(rule.id, 'scope', (e.target as HTMLSelectElement).value)}
      >
        {#each SCOPES as s}
          <option value={s.value} title={s.title}>{s.label}</option>
        {/each}
      </select>
    </div>
  </div>

  <div class="mode-toggles">
    <button
      class="mode-toggle"
      class:on={rule.case_sensitive}
      aria-pressed={rule.case_sensitive}
      onclick={() => updateRule(rule.id, 'case_sensitive', !rule.case_sensitive)}
      title={rule.case_sensitive ? 'Case sensitive' : 'Case insensitive'}
    >Aa</button>
    <button
      class="mode-toggle"
      class:on={rule.regex}
      aria-pressed={rule.regex}
      onclick={() => updateRule(rule.id, 'regex', !rule.regex)}
      title={rule.regex
        ? 'Regex mode — Contains is a regular expression'
        : 'Plain text mode — , means OR and * means AND'}
    >.*</button>
  </div>

  {#if matchCount > 0}
    <span class="match-badge" title="{matchCount} file{matchCount === 1 ? '' : 's'} match">{matchCount}</span>
  {/if}

  <button
    class="stop-toggle"
    aria-pressed={rule.stop_on_match}
    onclick={() => updateRule(rule.id, 'stop_on_match', !rule.stop_on_match)}
    title="Stop on Match — files matching this rule skip all later rules"
  >
    {#if rule.stop_on_match}
      <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
        <rect x="2" y="2" width="10" height="10" rx="2" fill="#ff3b30" />
      </svg>
    {:else}
      <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
        <path d="M7 2.5V10.5M7 10.5L3.5 7M7 10.5L10.5 7" stroke="white" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
    {/if}
  </button>

  <button class="icon-btn" onclick={() => duplicateRule(rule.id)} title="Duplicate rule">
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <rect x="1.5" y="1.5" width="8" height="8" rx="1.5" stroke="currentColor" stroke-width="1.4"/>
      <path d="M4.5 12.5H12C12.28 12.5 12.5 12.28 12.5 12V4.5" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>
    </svg>
  </button>

  <button class="delete-btn" onclick={() => removeRule(rule.id)} title="Remove rule">
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <path d="M2 2L12 12M12 2L2 12" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
    </svg>
  </button>
</div>

{#if ruleError}
  <div class="rule-error" role="alert">Rule {index + 1}: {ruleError}</div>
{/if}

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

  .rule-row.disabled {
    opacity: 0.5;
  }

  .toggle-switch {
    position: relative;
    display: inline-block;
    width: 30px;
    height: 17px;
    flex-shrink: 0;
    cursor: pointer;
  }

  .toggle-switch input {
    opacity: 0;
    width: 0;
    height: 0;
  }

  .toggle-slider {
    position: absolute;
    inset: 0;
    background: var(--border);
    border-radius: 999px;
    transition: background 0.15s;
  }

  .toggle-slider::before {
    content: '';
    position: absolute;
    height: 13px;
    width: 13px;
    left: 2px;
    top: 2px;
    background: white;
    border-radius: 50%;
    transition: transform 0.15s;
  }

  .toggle-switch input:checked + .toggle-slider {
    background: var(--accent);
  }

  .toggle-switch input:checked + .toggle-slider::before {
    transform: translateX(13px);
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
    /* Without this a flex item refuses to shrink below its content width, which
       pushed the trailing controls off the edge of a narrow panel. */
    min-width: 0;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    min-width: 0;
  }

  .field input {
    /* Below this the field stops being usable, so the panel gets a scrollbar
       instead of silently squeezing the text boxes to nothing. */
    min-width: 104px;
  }

  .field:nth-child(2),
  .field:nth-child(2) input {
    flex: 0.7;
    min-width: 76px;
  }

  .field-scope {
    flex: 0 0 auto;
  }

  .field-scope select {
    background: var(--surface-1);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 6px 4px;
    font-size: 12px;
    font-family: inherit;
    color: var(--text);
    outline: none;
    cursor: pointer;
  }

  .field-scope select:focus {
    border-color: var(--accent);
  }

  .mode-toggles {
    display: flex;
    gap: 3px;
    flex-shrink: 0;
    align-self: flex-end;
    padding-bottom: 1px;
  }

  .mode-toggle {
    width: 26px;
    height: 27px;
    background: var(--surface-1);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-muted);
    font-size: 11px;
    font-weight: 700;
    font-family: 'SF Mono', 'Cascadia Code', 'Fira Code', monospace;
    cursor: pointer;
    transition: color 0.15s, border-color 0.15s, background 0.15s;
  }

  .mode-toggle:hover {
    border-color: var(--border-hover);
    color: var(--text);
  }

  .mode-toggle.on {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }

  .icon-btn {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 4px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    transition: color 0.15s, background 0.15s;
  }

  .icon-btn:hover {
    color: var(--accent);
    background: var(--surface-1);
  }

  .rule-error {
    margin: 2px 0 4px 30px;
    padding: 4px 10px;
    font-size: 11px;
    color: var(--danger);
    background: rgba(255, 59, 48, 0.08);
    border-left: 2px solid var(--danger);
    border-radius: 4px;
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

  .stop-toggle {
    background: none;
    border: none;
    cursor: pointer;
    padding: 4px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    transition: background 0.15s;
  }

  .stop-toggle:hover {
    background: var(--surface-1);
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
