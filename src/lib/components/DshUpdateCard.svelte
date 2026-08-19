<script lang="ts">
  import Button from '$lib/components/Button.svelte';
  import StatusBadge from '$lib/components/StatusBadge.svelte';
  import TerminalLog from '$lib/components/TerminalLog.svelte';
  import type { DshInfo, UpdateInfo } from '$lib/api/types';

  export let dsh: DshInfo;
  export let update: UpdateInfo | null = null;
  export let checking = false;
  export let installing = false;
  export let installLog: string[] = [];
  export let onCheck: () => void = () => {};
  export let onInstall: () => void = () => {};

  // 安装中或检查中：操作按钮禁用
  $: busy = checking || installing;

  // 状态判定
  $: status = (!dsh.installed ? 'error' : update?.update_available ? 'warn' : 'ok') as 'ok' | 'warn' | 'error';
  $: statusLabel = !dsh.installed
    ? '未安装'
    : update?.update_available
      ? `有新版本 v${update.latest_version}`
      : dsh.version
        ? `已安装 v${dsh.version}`
        : '已安装';
</script>

<div
  class="rounded-lg border bg-white dark:bg-gray-800 p-5 transition-colors
  {status === 'error'
    ? 'border-red-300 dark:border-red-700'
    : status === 'warn'
      ? 'border-orange-300 dark:border-orange-700'
      : 'border-gray-200 dark:border-gray-700'}"
>
  <!-- 标题行 -->
  <div class="flex items-center justify-between gap-3">
    <div class="flex items-center gap-2">
      <svg class="h-4 w-4 text-gray-400 dark:text-gray-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M20 6 9 17l-5-5" />
      </svg>
      <span class="text-sm font-medium text-gray-700 dark:text-gray-300">dsh 更新</span>
    </div>
    <StatusBadge status={status} label={statusLabel} variant="pill" />
  </div>

  <!-- 路径 -->
  {#if dsh.path}
    <p class="mt-2 truncate text-xs text-gray-400 dark:text-gray-500 font-mono">{dsh.path}</p>
  {/if}

  <!-- 状态区 -->
  <div class="mt-3 space-y-2 text-xs text-gray-500 dark:text-gray-400">
    {#if checking}
      <p>正在检查最新版本...</p>
    {:else if !dsh.installed}
      <p>dsh 未安装，安装后即可启动 DeepSeek Harness。</p>
    {:else if update?.update_available}
      <p>
        当前 <span class="font-mono">v{update.current_version}</span> →
        最新 <span class="font-mono text-orange-600 dark:text-orange-400">v{update.latest_version}</span>
      </p>
    {:else if update}
      <p class="text-green-600 dark:text-green-400">✓ 已是最新版本</p>
    {/if}
  </div>

  <!-- 操作区 -->
  <div class="mt-4 flex flex-wrap items-center gap-2 border-t border-gray-100 dark:border-gray-700 pt-3">
    {#if !dsh.installed}
      <Button variant="primary" loading={installing} on:click={onInstall}>
        {installing ? '安装中...' : '安装 dsh'}
      </Button>
    {:else}
      <Button variant="secondary" loading={checking} icon={'<path d="M21 12a9 9 0 1 1-3-6.7L21 8"/><polyline points="21 3 21 8 16 8"/>'} on:click={onCheck}>
        {checking ? '检查中...' : '检查更新'}
      </Button>
      {#if update?.update_available}
        <Button variant="primary" loading={installing} on:click={onInstall}>
          {installing ? '更新中...' : `更新到 v${update.latest_version}`}
        </Button>
      {/if}
    {/if}
  </div>

  <!-- 安装/更新日志（可折叠） -->
  {#if installLog.length > 0}
    <div class="mt-4">
      <TerminalLog lines={installLog} title="安装日志" collapsible />
    </div>
  {/if}
</div>
