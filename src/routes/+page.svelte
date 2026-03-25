<script lang="ts">
  import PresetBar from '$lib/components/PresetBar.svelte';
  import RuleList from '$lib/components/RuleList.svelte';
  import DropZone from '$lib/components/DropZone.svelte';
  import StatusBar from '$lib/components/StatusBar.svelte';
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
