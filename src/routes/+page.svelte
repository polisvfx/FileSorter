<script lang="ts">
  import { onMount } from 'svelte';
  import PresetBar from '$lib/components/PresetBar.svelte';
  import RuleList from '$lib/components/RuleList.svelte';
  import DropZone from '$lib/components/DropZone.svelte';
  import StatusBar from '$lib/components/StatusBar.svelte';
  import { getSelectedPaths, setPaths, getOutputDir, setOutputDir, getCopyMode, setCopyMode } from '$lib/stores/app.svelte';
  import { getRules, setRules } from '$lib/stores/rules.svelte';
  import { loadSession, saveSession } from '$lib/stores/persistence';

  let initialized = false;

  onMount(() => {
    const session = loadSession();
    if (session) {
      if (session.rules?.length) setRules(session.rules);
      if (session.outputDir) setOutputDir(session.outputDir);
      if (session.copyMode) setCopyMode(session.copyMode);
      if (session.selectedPaths?.length) setPaths(session.selectedPaths);
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
</script>

<div class="app">
  <PresetBar />

  <main class="content">
    <div class="panel left-panel">
      <RuleList />
    </div>
    <div class="divider"></div>
    <div class="panel right-panel">
      <DropZone />
    </div>
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
    flex: 1.2;
    display: flex;
    flex-direction: column;
  }

  .divider {
    width: 1px;
    background: var(--border);
  }

  .right-panel {
    flex: 0.8;
    display: flex;
    flex-direction: column;
  }
</style>
