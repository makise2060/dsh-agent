<script lang="ts">
  import '../app.css';
  import TopNav from '$lib/components/TopNav.svelte';
  import Main from '$lib/views/Main.svelte';
  import EnvPanel from '$lib/views/EnvPanel.svelte';
  import AboutPanel from '$lib/views/AboutPanel.svelte';
  import PluginMarket from '$lib/views/PluginMarket.svelte';
  import CloseConfirm from '$lib/components/CloseConfirm.svelte';
  import { currentRoute } from '$lib/stores/app';
  import { theme } from '$lib/stores/theme';
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { listen } from '@tauri-apps/api/event';

  // 关闭确认框是否显示（后端 CloseRequested → close-requested 事件触发）
  let showCloseConfirm = false;
  let unlistenClose: (() => void) | undefined;

  onMount(() => {
    theme.init();

    // 防白屏：窗口以 visible:false 启动，前端首帧渲染完成后主动显示。
    // 若前端坏了永不 show，后端有 4 秒强制显示兜底。
    getCurrentWindow()
      .show()
      .catch(() => {});

    // 监听后端关闭请求，弹确认框
    listen('close-requested', () => {
      showCloseConfirm = true;
    }).then((fn) => {
      unlistenClose = fn;
    });

    return () => {
      unlistenClose?.();
    };
  });

  // For Tauri static SPA, we use client-side routing, not SvelteKit's router
  $: route = $currentRoute;
</script>

<div class="flex h-screen flex-col bg-white dark:bg-gray-900 text-gray-800 dark:text-gray-200 transition-colors">
  <TopNav />
  <main class="flex-1 overflow-hidden relative">
    <!-- 所有视图同时挂载，通过 opacity 切换可见性，避免重新挂载导致 iframe 重建/闪屏；
         同时加 transition 让切换有淡入淡出动画（display 切换无法过渡） -->
    <div class="absolute inset-0 transition-opacity duration-200 {$currentRoute === 'main' ? 'opacity-100' : 'opacity-0 pointer-events-none'}">
      <Main />
    </div>
    <div class="absolute inset-0 transition-opacity duration-200 {$currentRoute === 'env' ? 'opacity-100' : 'opacity-0 pointer-events-none'}">
      <EnvPanel />
    </div>
    <div class="absolute inset-0 transition-opacity duration-200 {$currentRoute === 'about' ? 'opacity-100' : 'opacity-0 pointer-events-none'}">
      <AboutPanel />
    </div>
    <div class="absolute inset-0 transition-opacity duration-200 {$currentRoute === 'plugins' ? 'opacity-100' : 'opacity-0 pointer-events-none'}">
      <PluginMarket />
    </div>
  </main>
</div>

{#if showCloseConfirm}
  <CloseConfirm
    onCancel={() => {
      showCloseConfirm = false;
    }}
  />
{/if}
