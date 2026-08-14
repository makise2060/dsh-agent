<script lang="ts">
  import '../app.css';
  import TopNav from '$lib/components/TopNav.svelte';
  import Main from '$lib/views/Main.svelte';
  import EnvPanel from '$lib/views/EnvPanel.svelte';
  import VersionPanel from '$lib/views/VersionPanel.svelte';
  import PluginMarket from '$lib/views/PluginMarket.svelte';
  import { currentRoute } from '$lib/stores/app';

  // For Tauri static SPA, we use client-side routing, not SvelteKit's router
  $: route = $currentRoute;
</script>

<div class="flex h-screen flex-col">
  <TopNav />
  <main class="flex-1 overflow-hidden">
    {#if route === 'main'}
      <Main />
    {:else if route === 'env'}
      <EnvPanel />
    {:else if route === 'version'}
      <VersionPanel />
    {:else if route === 'plugins'}
      <PluginMarket />
    {/if}
  </main>
</div>
