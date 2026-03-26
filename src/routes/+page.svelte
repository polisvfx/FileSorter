<script lang="ts">
  import { onMount } from 'svelte';
  import PresetBar from '$lib/components/PresetBar.svelte';
  import RuleList from '$lib/components/RuleList.svelte';
  import DropZone from '$lib/components/DropZone.svelte';
  import StatusBar from '$lib/components/StatusBar.svelte';
  import { getSelectedPaths, setPaths, getOutputDir, setOutputDir, getCopyMode, setCopyMode } from '$lib/stores/app.svelte';
  import { getRules, setRules } from '$lib/stores/rules.svelte';
  import { loadSession, saveSession } from '$lib/stores/persistence';

  const LEFT_PANEL_DEFAULT = 480;
  const LEFT_PANEL_MIN = 280;
  const RIGHT_PANEL_MIN = 300;
  const DIVIDER_KEY = 'filesorter-divider-width';

  let initialized = false;
  let leftPanelWidth = $state(LEFT_PANEL_DEFAULT);
  let isDragging = $state(false);

  onMount(() => {
    const session = loadSession();
    if (session) {
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

  $effect(() => {
    const state = {
      rules: getRules(),
      outputDir: getOutputDir(),
      copyMode: getCopyMode(),
      selectedPaths: getSelectedPaths()
    };
    if (initialized) {
      saveSession(state);
    }
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
    min-width: 280px;
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
    min-width: 300px;
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
