<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { onDshStdout, onBootstrapProgress, onBootstrapWarning, type BootstrapProgress } from '$lib/api/tauri';
  import TerminalLog from '$lib/components/TerminalLog.svelte';
  import { BOOTSTRAP_STEPS } from '$lib/components/steps';

  let logs: string[] = [];
  let progress: BootstrapProgress | null = null;
  let warnings: string[] = [];
  let unlistenStdout: (() => void) | null = null;
  let unlistenProgress: (() => void) | null = null;
  let unlistenWarning: (() => void) | null = null;

  // 当前到达的阶段下标（1-based：0 = 还没开始）
  let currentStage = 0;

  onMount(async () => {
    unlistenProgress = await onBootstrapProgress((p) => {
      progress = p;
      currentStage = p.index;
      // 非瞬态阶段说明已经完成，收进日志作为留痕
      if (!p.transient) {
        const detail = p.detail ? ` — ${p.detail}` : '';
        logs = [...logs, `[${p.label}]${detail}`];
      }
    });

    unlistenStdout = await onDshStdout((line: string) => {
      logs = [...logs, line];
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
</script>

<div class="flex h-full items-center justify-center bg-gray-50 dark:bg-gray-900 p-6">
  <!-- 整体卡片：左步骤条 + 右详情区 -->
  <div class="flex max-w-3xl min-h-0 h-full w-full overflow-hidden rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 shadow-sm">
    <!-- 左：纵向步骤条 -->
    <nav class="w-52 shrink-0 overflow-y-auto border-r border-gray-100 dark:border-gray-800 p-4">
      <ol class="space-y-1">
        {#each BOOTSTRAP_STEPS as step, i}
          <li class="relative flex items-start gap-3 py-1.5">
            <!-- 连接线 -->
            {#if i < BOOTSTRAP_STEPS.length - 1}
              <span
                class="absolute left-[15px] top-9 bottom-[-6px] w-px {i < currentStage
                  ? 'bg-green-400 dark:bg-green-600'
                  : 'bg-gray-200 dark:bg-gray-700'}"
              ></span>
            {/if}
            <!-- 图标圈 -->
            <span
              class="relative z-10 flex h-8 w-8 shrink-0 items-center justify-center rounded-full transition-colors
              {i < currentStage
                ? 'bg-green-500 text-white'
                : i === currentStage
                  ? 'bg-brand-50 dark:bg-brand-900/40 text-brand-600 dark:text-brand-400 border border-brand-200 dark:border-brand-700 animate-pulse'
                  : 'bg-gray-100 dark:bg-gray-700/60 text-gray-400 dark:text-gray-500'}"
            >
              {#if i < currentStage}
                <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                  <polyline points="20 6 9 17 4 12" />
                </svg>
              {:else}
                <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  {@html step.icon}
                </svg>
              {/if}
            </span>
            <!-- 标签 -->
            <div class="min-w-0 pt-1">
              <p
                class="text-xs leading-tight
                {i < currentStage
                  ? 'font-medium text-green-600 dark:text-green-400'
                  : i === currentStage
                    ? 'font-bold text-brand-600 dark:text-brand-400'
                    : 'text-gray-400 dark:text-gray-500'}"
              >
                {step.label}
              </p>
            </div>
          </li>
        {/each}
      </ol>
    </nav>

    <!-- 右：当前阶段详情区 -->
    <section class="flex min-w-0 flex-1 flex-col gap-4 overflow-y-auto p-6">
      <!-- 大标题 + 步骤计数 -->
      <div>
        <div class="flex items-baseline justify-between gap-4">
          <h2 class="text-lg font-semibold text-gray-800 dark:text-gray-200">
            {progress ? progress.label : '正在启动 DeepSeek Harness...'}
          </h2>
          {#if progress}
            <span class="shrink-0 text-xs text-gray-400 dark:text-gray-500">
              步骤 {progress.index}/{progress.total}
            </span>
          {/if}
        </div>
        <!-- 瞬态详情（下载字节数 / 安装计数等） -->
        {#if progress?.detail}
          <p class="mt-1.5 font-mono text-xs text-gray-500 dark:text-gray-400">{progress.detail}</p>
        {/if}
        <!-- 进度条 -->
        {#if progress?.fraction != null}
          <div class="mt-3 h-2 w-full overflow-hidden rounded-full bg-gray-200 dark:bg-gray-700">
            <div
              class="h-full rounded-full bg-brand-600 transition-all duration-300"
              style="width: {Math.min(100, progress.fraction * 100)}%"
            ></div>
          </div>
        {/if}
      </div>

      <!-- 非致命警告 -->
      {#if warnings.length > 0}
        <div class="rounded-md border border-orange-200 dark:border-orange-800 bg-orange-50 dark:bg-orange-900/30 px-3 py-2">
          {#each warnings as w}
            <p class="text-xs text-orange-700 dark:text-orange-400">{w}</p>
          {/each}
        </div>
      {/if}

      <!-- 日志面板（深色终端拟物，作为唯一深色焦点） -->
      <div class="min-h-0">
        <TerminalLog lines={logs} title="引导日志" maxHeight="max-h-56" />
      </div>
    </section>
  </div>
</div>
