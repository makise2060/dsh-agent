<script lang="ts">
  import { onMount } from 'svelte';
  import { envState } from '$lib/stores/env';
  import { checkDshUpdate, checkAppUpdate, installDsh, onInstallProgress } from '$lib/api/tauri';
  import type { UpdateInfo } from '$lib/api/types';

  let dshUpdate: UpdateInfo | null = null;
  let appUpdate: UpdateInfo | null = null;
  let checkingDsh = false;
  let checkingApp = false;
  let installing = false;
  let installLog = '';

  onMount(async () => {
    const unlisten = await onInstallProgress((p) => {
      installLog += p.message + '\n';
      if (p.stage === 'done') {
        installing = false;
      }
    });
    return () => unlisten();
  });

  async function handleCheckDsh() {
    checkingDsh = true;
    try {
      dshUpdate = await checkDshUpdate();
    } catch (e) {
      console.error(e);
    } finally {
      checkingDsh = false;
    }
  }

  async function handleCheckApp() {
    checkingApp = true;
    try {
      appUpdate = await checkAppUpdate();
    } catch (e) {
      console.error(e);
    } finally {
      checkingApp = false;
    }
  }

  async function handleInstallDsh() {
    installing = true;
    installLog = '';
    try {
      await installDsh();
      await handleCheckDsh();
    } catch (e) {
      installLog += `Error: ${e}\n`;
      installing = false;
    }
  }

  $: dsh = $envState.dsh;
</script>

<div class="h-full overflow-y-auto bg-gray-50 dark:bg-gray-900 p-6 transition-colors">
  <div class="mx-auto max-w-2xl space-y-6">
    <h1 class="text-lg font-bold text-gray-800 dark:text-gray-200">版本与更新</h1>

    <!-- App self update -->
    <div class="rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 p-4 transition-colors">
      <div class="flex items-center justify-between">
        <span class="text-sm font-medium text-gray-700 dark:text-gray-300">DSH Agent (本应用)</span>
        <span class="text-xs text-gray-400 dark:text-gray-500">v1.1.3</span>
      </div>
      <div class="mt-3">
        {#if checkingApp}
          <span class="text-xs text-gray-400 dark:text-gray-500">检查中...</span>
        {:else if appUpdate?.update_available}
          <p class="text-xs text-orange-600 dark:text-orange-400">最新版本: {appUpdate.latest_version}</p>
        {:else if appUpdate}
          <span class="text-xs text-green-600 dark:text-green-400">已是最新版本</span>
        {/if}
      </div>
      <button
        class="mt-3 rounded-md border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 px-3 py-1.5 text-xs font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
        on:click={handleCheckApp}
        disabled={checkingApp}
      >
        检查更新
      </button>
    </div>

    <!-- dsh update -->
    <div class="rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 p-4 transition-colors">
      <div class="flex items-center justify-between">
        <span class="text-sm font-medium text-gray-700 dark:text-gray-300">dsh (DeepSeek Harness)</span>
        {#if dsh.installed}
          <span class="text-xs text-gray-400 dark:text-gray-500">v{dsh.version}</span>
        {:else}
          <span class="text-xs text-red-500 dark:text-red-400">未安装</span>
        {/if}
      </div>
      <div class="mt-3">
        {#if checkingDsh}
          <span class="text-xs text-gray-400 dark:text-gray-500">检查中...</span>
        {:else if dshUpdate?.update_available}
          <p class="text-xs text-orange-600 dark:text-orange-400">
            最新版本: {dshUpdate.latest_version} (有新版本)
          </p>
        {:else if dshUpdate}
          <span class="text-xs text-green-600 dark:text-green-400">已是最新版本</span>
        {/if}
      </div>
      <div class="mt-3 flex gap-2">
        <button
          class="rounded-md border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 px-3 py-1.5 text-xs font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
          on:click={handleCheckDsh}
          disabled={checkingDsh}
        >
          检查更新
        </button>
        {#if dshUpdate?.update_available}
          <button
            class="rounded-md bg-brand-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-brand-700"
            on:click={handleInstallDsh}
            disabled={installing}
          >
            {installing ? '更新中...' : `更新到 ${dshUpdate.latest_version}`}
          </button>
        {/if}
      </div>
      {#if installLog}
        <pre class="mt-3 max-h-32 overflow-y-auto rounded bg-gray-900 p-2 text-xs text-green-400">{installLog}</pre>
      {/if}
    </div>
  </div>
</div>
