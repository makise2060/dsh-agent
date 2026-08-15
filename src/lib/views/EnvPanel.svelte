<script lang="ts">
  import { onMount } from 'svelte';
  import { envState, envChecking } from '$lib/stores/env';
  import { checkEnvironment, installDsh, checkDshUpdate, onInstallProgress } from '$lib/api/tauri';
  import type { UpdateInfo } from '$lib/api/types';
  import StatusBadge from '$lib/components/StatusBadge.svelte';

  let dshUpdate: UpdateInfo | null = null;
  let checkingDsh = false;
  let installing = false;
  let installLog = '';

  onMount(() => {
    refresh();
    let unlisten: (() => void) | undefined;

    onInstallProgress((p) => {
      installLog += p.message + '\n';
      if (p.stage === 'done') {
        installing = false;
        refresh();
      }
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
    };
  });

  async function refresh() {
    envChecking.set(true);
    try {
      const state = await checkEnvironment();
      envState.set(state);
      // Also refresh dsh update info if it was checked before
      if (dshUpdate) {
        await handleCheckDsh();
      }
    } catch (e) {
      console.error('Env check failed:', e);
    } finally {
      envChecking.set(false);
    }
  }

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

  async function handleInstallDsh() {
    installing = true;
    installLog = '';
    try {
      await installDsh();
      await refresh();
      await handleCheckDsh();
    } catch (e) {
      installLog += `Error: ${e}\n`;
      installing = false;
    }
  }

  $: env = $envState;
</script>

<div class="h-full overflow-y-auto bg-gray-50 dark:bg-gray-900 p-6 transition-colors">
  <div class="mx-auto max-w-2xl space-y-6">
    <h1 class="text-lg font-bold text-gray-800 dark:text-gray-200">运行环境</h1>

    <!-- Node.js -->
    <div class="rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 p-4 transition-colors">
      <div class="flex items-center justify-between">
        <span class="text-sm font-medium text-gray-700 dark:text-gray-300">Node.js</span>
        {#if env.node.installed}
          {#if env.node.meets_minimum}
            <StatusBadge status="ok" label={`v${env.node.version}`} />
          {:else}
            <StatusBadge status="warn" label={`v${env.node.version} (需 ≥22)`} />
          {/if}
        {:else}
          <StatusBadge status="error" label="未安装" />
        {/if}
      </div>
      {#if env.node.path}
        <p class="mt-2 truncate text-xs text-gray-400 dark:text-gray-500">{env.node.path}</p>
      {/if}
      {#if !env.node.installed || !env.node.meets_minimum}
        <a
          href="https://nodejs.org/"
          target="_blank"
          rel="noopener"
          class="mt-2 inline-block text-xs text-brand-600 dark:text-brand-400 hover:underline"
        >
          下载 Node.js →
        </a>
      {/if}
    </div>

    <!-- npm -->
    <div class="rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 p-4 transition-colors">
      <div class="flex items-center justify-between">
        <span class="text-sm font-medium text-gray-700 dark:text-gray-300">npm</span>
        {#if env.npm.installed}
          <StatusBadge status="ok" label={`v${env.npm.version}`} />
        {:else}
          <StatusBadge status="error" label="未安装" />
        {/if}
      </div>
      {#if env.npm.path}
        <p class="mt-2 truncate text-xs text-gray-400 dark:text-gray-500">{env.npm.path}</p>
      {/if}
    </div>

    <!-- dsh (with update check) -->
    <div class="rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 p-4 transition-colors">
      <div class="flex items-center justify-between">
        <span class="text-sm font-medium text-gray-700 dark:text-gray-300">dsh (DeepSeek Harness)</span>
        {#if env.dsh.installed}
          {#if env.dsh.update_available}
            <StatusBadge status="warn" label={`v${env.dsh.version} (有更新)`} />
          {:else}
            <StatusBadge status="ok" label={`v${env.dsh.version}`} />
          {/if}
        {:else}
          <StatusBadge status="error" label="未安装" />
        {/if}
      </div>
      {#if env.dsh.path}
        <p class="mt-2 truncate text-xs text-gray-400 dark:text-gray-500">{env.dsh.path}</p>
      {/if}

      <!-- Update status -->
      {#if checkingDsh}
        <p class="mt-2 text-xs text-gray-400 dark:text-gray-500">检查更新中...</p>
      {:else if dshUpdate?.update_available}
        <p class="mt-2 text-xs text-orange-600 dark:text-orange-400">
          最新版本: {dshUpdate.latest_version} (有新版本)
        </p>
      {:else if dshUpdate}
        <p class="mt-2 text-xs text-green-600 dark:text-green-400">✓ dsh 已是最新版本</p>
      {/if}

      <!-- Action buttons -->
      <div class="mt-3 flex flex-wrap gap-2">
        {#if !env.dsh.installed}
          <button
            class="inline-flex items-center gap-1.5 rounded-md bg-brand-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-brand-700 transition-colors"
            on:click={handleInstallDsh}
            disabled={installing}
          >
            {#if installing}
              <span class="h-3 w-3 animate-spin rounded-full border-2 border-white border-t-transparent"></span>
              安装中...
            {:else}
              <svg class="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
              一键安装 dsh
            {/if}
          </button>
        {:else}
          <button
            class="inline-flex items-center gap-1.5 rounded-md border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 px-3 py-1.5 text-xs font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors disabled:opacity-50"
            on:click={handleCheckDsh}
            disabled={checkingDsh}
          >
            <svg class="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"/><polyline points="1 20 1 14 7 14"/><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/></svg>
            {checkingDsh ? '检查中...' : '检查更新'}
          </button>
          {#if dshUpdate?.update_available || env.dsh.update_available}
            <button
              class="inline-flex items-center gap-1.5 rounded-md bg-brand-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-brand-700 transition-colors disabled:opacity-50"
              on:click={handleInstallDsh}
              disabled={installing}
            >
              {#if installing}
                <span class="h-3 w-3 animate-spin rounded-full border-2 border-white border-t-transparent"></span>
                更新中...
              {:else}
                <svg class="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
                更新到 {dshUpdate?.latest_version || env.dsh.latest_version}
              {/if}
            </button>
          {/if}
        {/if}
      </div>

      {#if installLog}
        <pre class="mt-3 max-h-32 overflow-y-auto rounded bg-gray-900 p-2 text-xs text-green-400 font-mono whitespace-pre-wrap break-all">{installLog}</pre>
      {/if}
    </div>

    <!-- DSH_HOME -->
    <div class="rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 p-4 transition-colors">
      <div class="flex items-center justify-between">
        <span class="text-sm font-medium text-gray-700 dark:text-gray-300">DSH_HOME</span>
        {#if env.dsh_home.exists}
          <StatusBadge status="ok" label="就绪" />
        {:else}
          <StatusBadge status="warn" label="未创建（首次运行自动创建）" />
        {/if}
      </div>
      <p class="mt-2 truncate text-xs text-gray-400 dark:text-gray-500">{env.dsh_home.path}</p>
      {#if env.dsh_home.exists}
        <p class="mt-1 text-xs text-gray-400 dark:text-gray-500">
          profiles: {env.dsh_home.profiles_dir ? '✅' : '❌'} ·
          sessions: {env.dsh_home.sessions_dir ? '✅' : '❌'}
        </p>
      {/if}
    </div>

    <!-- Actions -->
    <div class="flex gap-3 pt-2">
      <button
        class="inline-flex items-center gap-1.5 rounded-md border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 px-4 py-2 text-xs font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors disabled:opacity-50"
        on:click={refresh}
        disabled={$envChecking}
      >
        <svg class="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"/><polyline points="1 20 1 14 7 14"/><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/></svg>
        {$envChecking ? '检查中...' : '重新检查'}
      </button>
    </div>
  </div>
</div>
