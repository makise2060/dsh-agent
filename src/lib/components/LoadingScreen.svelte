<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { onDshStdout, onBootstrapProgress, onBootstrapWarning, type BootstrapProgress } from '$lib/api/tauri';
  import { BOOTSTRAP_STEPS } from '$lib/components/steps';

  let logs: string[] = [];
  let progress: BootstrapProgress | null = null;
  let warnings: string[] = [];
  let unlistenStdout: (() => void) | null = null;
  let unlistenProgress: (() => void) | null = null;
  let unlistenWarning: (() => void) | null = null;

  // 当前阶段下标（0-based，由事件 index-1 得来）
  let current = 0;

  onMount(async () => {
    unlistenProgress = await onBootstrapProgress((p) => {
      progress = p;
      current = Math.max(0, p.index - 1);
      // 非瞬态阶段说明已经完成，收进日志作为留痕
      if (!p.transient) {
        const detail = p.detail ? ` — ${p.detail}` : '';
        logs = [...logs, `[${p.label}]${detail}`];
      }
    });

    unlistenStdout = await onDshStdout((line: string) => {
      logs = [...logs, line];
      setTimeout(() => {
        const el = document.getElementById('dsh-log-scroll');
        if (el) el.scrollTop = el.scrollHeight;
      }, 0);
    });

    unlistenWarning = await onBootstrapWarning((w) => {
      warnings = [...warnings, w.message];
    });
  });

  onDestroy(() => {
    unlistenStdout?.();
    unlistenProgress?.();
    unlistenWarning?.();
  });

  $: step = BOOTSTRAP_STEPS[current] ?? BOOTSTRAP_STEPS[0];

  // 整体进度（自己计算）：已完成阶段 + 当前阶段内 fraction，映射 0~100
  $: overallPercent = progress
    ? Math.round(((progress.index - 1 + (progress.fraction ?? 0)) / progress.total) * 100)
    : 0;
</script>

<div class="flex h-full flex-col items-center justify-center gap-5 bg-gray-50 dark:bg-gray-900 p-8">
  <!-- 主卡片 -->
  <div class="w-full max-w-md rounded-2xl border border-gray-200/80 dark:border-gray-700/80 bg-white dark:bg-gray-800 p-7 shadow-sm transition-colors">
    <!-- 当前步骤：icon + 名称 -->
    <div class="flex flex-col items-center text-center">
      <div
        class="flex h-14 w-14 items-center justify-center rounded-2xl bg-brand-50 dark:bg-brand-900/40 text-brand-600 dark:text-brand-400"
      >
        <svg class="h-7 w-7" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          {@html step.icon}
        </svg>
      </div>
      <h2 class="mt-4 text-base font-semibold text-gray-800 dark:text-gray-100">
        {progress ? progress.label : '正在启动 DeepSeek Harness...'}
      </h2>
      {#if progress?.detail}
        <p class="mt-1 text-sm text-gray-500 dark:text-gray-400">{progress.detail}</p>
      {/if}
    </div>

    <!-- 整体进度条（始终显示，随阶段推进与 fraction 平滑前进） -->
    <div class="mt-6">
      <div class="h-2 w-full overflow-hidden rounded-full bg-gray-100 dark:bg-gray-700/60">
        <div
          class="h-full rounded-full bg-gradient-to-r from-brand-500 to-brand-600 transition-all duration-500"
          style="width: {overallPercent}%"
        ></div>
      </div>
      <p class="mt-2 text-center text-xs text-gray-400 dark:text-gray-500">
        {overallPercent}%
      </p>
    </div>

    <!-- 非致命警告 -->
    {#if warnings.length > 0}
      <div class="mt-5 rounded-lg border border-orange-200 dark:border-orange-800 bg-orange-50 dark:bg-orange-900/30 px-3 py-2">
        {#each warnings as w}
          <p class="text-xs text-orange-700 dark:text-orange-400">{w}</p>
        {/each}
      </div>
    {/if}
  </div>

  <!-- 引导日志（有内容才显示，快速检查时几乎无感） -->
  {#if logs.length > 0}
    <div class="w-full max-w-md overflow-hidden rounded-xl border border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900">
      <div class="flex items-center justify-between border-b border-gray-200 dark:border-gray-700 px-4 py-2">
        <span class="text-xs font-medium text-gray-500 dark:text-gray-400">引导日志</span>
        <span class="text-[10px] text-gray-400 dark:text-gray-500">{logs.length} 行</span>
      </div>
      <div id="dsh-log-scroll" class="max-h-32 overflow-y-auto p-3 space-y-1">
        {#each logs as log}
          <pre class="text-xs text-gray-600 dark:text-gray-300 whitespace-pre-wrap break-all leading-relaxed">{log}</pre>
        {/each}
      </div>
    </div>
  {/if}
</div>
