<script lang="ts">
  import { currentRoute, processState, type NavRoute } from '$lib/stores/app';
  import { onMount } from 'svelte';
  import { onProcessStateChanged } from '$lib/api/tauri';

  const tabs: { id: NavRoute; label: string }[] = [
    { id: 'main', label: '主界面' },
    { id: 'env', label: '运行环境' },
    { id: 'plugins', label: '插件市场' },
    { id: 'about', label: '关于' }
  ];

  const statusColors: Record<string, { dot: string; text: string; label: string }> = {
    NotStarted: { dot: 'bg-gray-400', text: 'text-gray-500 dark:text-gray-400', label: '未启动' },
    Starting: { dot: 'bg-yellow-400 animate-pulse', text: 'text-yellow-600 dark:text-yellow-400', label: '启动中' },
    Running: { dot: 'bg-green-500', text: 'text-green-600 dark:text-green-400', label: '运行中' },
    Stopping: { dot: 'bg-yellow-400 animate-pulse', text: 'text-yellow-600 dark:text-yellow-400', label: '停止中' },
    Stopped: { dot: 'bg-gray-400', text: 'text-gray-500 dark:text-gray-400', label: '已停止' },
    Failed: { dot: 'bg-red-500', text: 'text-red-600 dark:text-red-400', label: '错误' }
  };

  $: status = $processState.status;
  $: statusInfo = statusColors[status] ?? statusColors.NotStarted;

  onMount(() => {
    const unlisten = onProcessStateChanged((state) => {
      processState.set(state);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  });
</script>

<nav class="flex h-10 items-center justify-between border-b border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 px-4 shrink-0 transition-colors">
  <!-- Left: App name + tabs -->
  <div class="flex items-center gap-4">
    <span class="text-sm font-bold text-gray-800 dark:text-gray-200">DSH Agent</span>
    <div class="flex items-center gap-1">
      {#each tabs as tab}
        <button
          class="rounded px-3 py-1 text-xs font-medium transition-colors {$currentRoute === tab.id
            ? 'bg-brand-100 dark:bg-brand-900/50 text-brand-700 dark:text-brand-300'
            : 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700'}"
          on:click={() => currentRoute.set(tab.id)}
        >
          {tab.label}
        </button>
      {/each}
    </div>
  </div>

  <!-- Right: Status indicator -->
  <div class="flex items-center gap-2">
    <span class="inline-block h-2 w-2 rounded-full {statusInfo.dot}"></span>
    <span class="text-xs {statusInfo.text}">{statusInfo.label}</span>
  </div>
</nav>
