<script lang="ts">
  import { currentRoute, processState, type NavRoute } from '$lib/stores/app';
  import { onMount } from 'svelte';
  import { onProcessStateChanged } from '$lib/api/tauri';

  const tabs: { id: NavRoute; label: string }[] = [
    { id: 'main', label: '主界面' },
    { id: 'env', label: '运行环境' },
    { id: 'version', label: '版本更新' },
    { id: 'plugins', label: '插件市场' }
  ];

  const statusColors: Record<string, { dot: string; text: string; label: string }> = {
    NotStarted: { dot: 'bg-gray-400', text: 'text-gray-500', label: '未启动' },
    Starting: { dot: 'bg-yellow-400 animate-pulse', text: 'text-yellow-600', label: '启动中' },
    Running: { dot: 'bg-green-500', text: 'text-green-600', label: '运行中' },
    Stopping: { dot: 'bg-yellow-400 animate-pulse', text: 'text-yellow-600', label: '停止中' },
    Stopped: { dot: 'bg-gray-400', text: 'text-gray-500', label: '已停止' },
    Failed: { dot: 'bg-red-500', text: 'text-red-600', label: '错误' }
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

<nav class="flex h-10 items-center justify-between border-b border-gray-200 bg-white px-4 shrink-0">
  <!-- Left: App name + tabs -->
  <div class="flex items-center gap-4">
    <span class="text-sm font-bold text-gray-800">DSH Agent</span>
    <div class="flex items-center gap-1">
      {#each tabs as tab}
        <button
          class="rounded px-3 py-1 text-xs font-medium transition-colors {$currentRoute === tab.id
            ? 'bg-brand-100 text-brand-700'
            : 'text-gray-600 hover:bg-gray-100'}"
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
