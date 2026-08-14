<script lang="ts">
  import { onMount } from 'svelte';
  import { envState, envChecking } from '$lib/stores/env';
  import { checkEnvironment, installDsh, onInstallProgress } from '$lib/api/tauri';
  import StatusBadge from '$lib/components/StatusBadge.svelte';

  onMount(async () => {
    await refresh();
    const unlisten = await onInstallProgress(() => {
      // re-check env after install completes
      refresh();
    });
    return () => unlisten();
  });

  async function refresh() {
    envChecking.set(true);
    try {
      const state = await checkEnvironment();
      envState.set(state);
    } catch (e) {
      console.error('Env check failed:', e);
    } finally {
      envChecking.set(false);
    }
  }

  async function handleInstallDsh() {
    try {
      await installDsh();
      await refresh();
    } catch (e) {
      console.error('Install failed:', e);
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

    <!-- dsh -->
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
      {#if !env.dsh.installed}
        <button
          class="mt-2 rounded-md bg-brand-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-brand-700"
          on:click={handleInstallDsh}
        >
          一键安装 dsh
        </button>
      {:else if env.dsh.update_available && env.dsh.latest_version}
        <button
          class="mt-2 rounded-md bg-brand-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-brand-700"
          on:click={handleInstallDsh}
        >
          更新到 {env.dsh.latest_version}
        </button>
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
        class="rounded-md border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 px-4 py-2 text-xs font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
        on:click={refresh}
        disabled={$envChecking}
      >
        {$envChecking ? '检查中...' : '重新检查'}
      </button>
    </div>
  </div>
</div>
