<script lang="ts">
  import { onMount } from 'svelte';
  import PresetBar from '$lib/components/PresetBar.svelte';
  import RuleList from '$lib/components/RuleList.svelte';
  import DropZone from '$lib/components/DropZone.svelte';
  import StatusBar from '$lib/components/StatusBar.svelte';
  import { getRootPaths, setPaths, getOutputDir, setOutputDir, getCopyMode, setCopyMode } from '$lib/stores/app.svelte';
  import { getRules, setRules } from '$lib/stores/rules.svelte';
  import { loadSession, saveSession } from '$lib/stores/persistence';

  // Measured against a rule row carrying every control (fields, scope, mode
  // toggles, stop/duplicate/delete): below 659px the row starts clipping, so the
  // minimum is set just above that and the default leaves the text fields room.
  const LEFT_PANEL_DEFAULT = 720;
  const LEFT_PANEL_MIN = 660;
  // Enough for the action row with Cancel showing mid-sort.
  const RIGHT_PANEL_MIN = 520;
  const DIVIDER_KEY = 'filesorter-divider-width';
  const SAVE_DEBOUNCE_MS = 200;

  let initialized = false;
  let leftPanelWidth = $state(LEFT_PANEL_DEFAULT);
  let isDragging = $state(false);

  onMount(() => {
    const session = loadSession();
    if (session) {
      // setRules fills in fields added since the session was written.
      if (session.rules?.length) setRules(session.rules);
      if (session.outputDir) setOutputDir(session.outputDir);
      if (session.copyMode) setCopyMode(session.copyMode);
      if (session.selectedPaths?.length) setPaths(session.selectedPaths);
    }

    const saved = localStorage.getItem(DIVIDER_KEY);
    if (saved) {
      const parsed = parseInt(saved, 10);
      if (!isNaN(parsed) && parsed >= LEFT_PANEL_MIN) {
        leftPanelWidth = parsed;
      }
    }

    initialized = true;
  });

  // Debounced: the selected-path list can hold every file under a dropped folder,
  // and this used to re-serialize the whole thing on every keystroke in a rule.
  $effect(() => {
    const state = {
      rules: getRules().map((r) => ({ ...r })),
      outputDir: getOutputDir(),
      copyMode: getCopyMode(),
      selectedPaths: getRootPaths()
    };
    if (!initialized) return;

    const timer = setTimeout(() => saveSession(state), SAVE_DEBOUNCE_MS);
    return () => clearTimeout(timer);
  });

  $effect(() => {
    if (initialized) {
      localStorage.setItem(DIVIDER_KEY, String(leftPanelWidth));
    }
  });

  function onDividerMouseDown(e: MouseEvent) {
    e.preventDefault();
    isDragging = true;

    const startX = e.clientX;
    const startWidth = leftPanelWidth;

    function onMouseMove(e: MouseEvent) {
      const delta = e.clientX - startX;
      const container = document.querySelector('.content') as HTMLElement;
      const maxLeft = container.clientWidth - RIGHT_PANEL_MIN;
      leftPanelWidth = Math.max(LEFT_PANEL_MIN, Math.min(maxLeft, startWidth + delta));
    }

    function onMouseUp() {
      isDragging = false;
      window.removeEventListener('mousemove', onMouseMove);
      window.removeEventListener('mouseup', onMouseUp);
    }

    window.addEventListener('mousemove', onMouseMove);
    window.addEventListener('mouseup', onMouseUp);
  }
</script>

<div class="app">
  <PresetBar />

  <main class="content">
    <div class="panel left-panel" style="width: {leftPanelWidth}px">
      <RuleList />
    </div>
    <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
    <div
      class="divider"
      class:dragging={isDragging}
      onmousedown={onDividerMouseDown}
      role="separator"
      tabindex="0"
      aria-orientation="vertical"
    ></div>
    <div class="panel right-panel">
      <DropZone />
    </div>
    {#if isDragging}
      <div class="drag-overlay"></div>
    {/if}
  </main>

  <StatusBar />
</div>

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg);
    color: var(--text);
  }

  .content {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .panel {
    padding: 16px;
    overflow-y: auto;
  }

  .left-panel {
    min-width: 660px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
  }

  .divider {
    width: 1px;
    background: var(--border);
    cursor: col-resize;
    position: relative;
    flex-shrink: 0;
  }

  .divider::before {
    content: '';
    position: absolute;
    top: 0;
    bottom: 0;
    left: -3px;
    width: 7px;
    z-index: 1;
  }

  .divider:hover,
  .divider.dragging {
    background: var(--accent);
  }

  .right-panel {
    flex: 1;
    min-width: 520px;
    display: flex;
    flex-direction: column;
  }

  .drag-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 9999;
    cursor: col-resize;
  }
</style>
