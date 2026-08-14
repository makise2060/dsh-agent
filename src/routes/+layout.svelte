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
  <main class="flex-1 overflow-hidden relative">
    <!-- 所有视图同时挂载，通过 display 切换可见性，避免重新挂载导致 iframe 重建/闪屏 -->
    <div class="absolute inset-0 {$currentRoute === 'main' ? 'block' : 'hidden'}">
      <Main />
    </div>
    <div class="absolute inset-0 {$currentRoute === 'env' ? 'block' : 'hidden'}">
      <EnvPanel />
    </div>
    <div class="absolute inset-0 {$currentRoute === 'version' ? 'block' : 'hidden'}">
      <VersionPanel />
    </div>
    <div class="absolute inset-0 {$currentRoute === 'plugins' ? 'block' : 'hidden'}">
      <PluginMarket />
    </div>
  </main>
</div>
