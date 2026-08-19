<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { onDshStdout, onBootstrapProgress, onBootstrapWarning, type BootstrapProgress } from '$lib/api/tauri';

  let logs: string[] = [];
  let progress: BootstrapProgress | null = null;
  let warnings: string[] = [];
  let unlistenStdout: (() => void) | null = null;
  let unlistenProgress: (() => void) | null = null;
  let unlistenWarning: (() => void) | null = null;

  // 9 阶段里程碑（与 Rust 端 Stage 顺序一致）
  const stages = [
    { key: 'checkingNode', label: '检查 Node.js 环境' },
    { key: 'downloadingNode', label: '下载 Node.js 运行时' },
    { key: 'checkingDsh', label: '检查 dsh 版本' },
    { key: 'installingDsh', label: '安装 DeepSeek Harness' },
    { key: 'initProfile', label: '初始化配置' },
    { key: 'installingPlugins', label: '安装界面插件' },
    { key: 'verifyingPlugins', label: '校验插件挂载' },
    { key: 'startingDsh', label: '启动 dsh 服务' },
    { key: 'waitingReady', label: '等待服务就绪' }
  ];

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
</script>

<div class="flex h-full flex-col items-center justify-center gap-5 bg-gray-50 dark:bg-gray-900 p-8">
  <!-- Spinner + Title -->
  <div class="flex items-center gap-3">
    <div class="h-6 w-6 animate-spin rounded-full border-2 border-brand-500 border-t-transparent"></div>
    <span class="text-sm font-medium text-gray-700 dark:text-gray-300">
      {progress ? progress.label : '正在启动 DeepSeek Harness...'}
    </span>
  </div>

  <!-- 9 阶段里程碑 -->
  <div class="w-full max-w-md space-y-2">
    {#each stages as stage, i}
      <div class="flex items-start gap-3">
        <div
          class="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full text-xs font-medium
          {i < currentStage ? 'bg-green-500 text-white' : ''}
          {i === currentStage ? 'bg-brand-500 text-white animate-pulse' : ''}
          {i > currentStage ? 'bg-gray-200 dark:bg-gray-700 text-gray-400 dark:text-gray-500' : ''}"
        >
          {#if i < currentStage}✓{:else}{i + 1}{/if}
        </div>
        <div class="flex-1">
          <p
            class="text-xs font-medium
            {i < currentStage ? 'text-green-600 dark:text-green-400' : ''}
            {i === currentStage ? 'text-brand-600 dark:text-brand-400' : ''}
            {i > currentStage ? 'text-gray-400 dark:text-gray-500' : ''}"
          >
            {stage.label}
          </p>
          {#if i === currentStage && progress?.detail}
            <p class="mt-0.5 text-xs text-gray-400 dark:text-gray-500 font-mono">{progress.detail}</p>
          {/if}
        </div>
        {#if i === currentStage && progress?.fraction != null}
          <div class="mt-1 h-1.5 w-16 shrink-0 overflow-hidden rounded-full bg-gray-200 dark:bg-gray-700">
            <div
              class="h-full bg-brand-600 transition-all duration-300"
              style="width: {Math.min(100, progress.fraction * 100)}%"
            ></div>
          </div>
        {/if}
      </div>
    {/each}
  </div>

  <!-- 非致命警告 -->
  {#if warnings.length > 0}
    <div class="w-full max-w-md rounded-md border border-orange-200 dark:border-orange-800 bg-orange-50 dark:bg-orange-900/30 px-3 py-2">
      {#each warnings as w}
        <p class="text-xs text-orange-700 dark:text-orange-400">{w}</p>
      {/each}
    </div>
  {/if}

  <!-- Live log output -->
  {#if logs.length > 0}
    <div class="w-full max-w-md">
      <div class="rounded-lg bg-gray-900 dark:bg-black border border-gray-700 dark:border-gray-800 overflow-hidden">
        <div class="flex items-center gap-2 px-3 py-1.5 border-b border-gray-700 dark:border-gray-800">
          <div class="flex gap-1.5">
            <span class="h-2.5 w-2.5 rounded-full bg-red-400"></span>
            <span class="h-2.5 w-2.5 rounded-full bg-yellow-400"></span>
            <span class="h-2.5 w-2.5 rounded-full bg-green-400"></span>
          </div>
          <span class="text-xs text-gray-400 ml-1">引导日志</span>
        </div>
        <div id="dsh-log-scroll" class="max-h-40 overflow-y-auto p-3 space-y-0.5">
          {#each logs as log}
            <pre class="text-xs text-green-400 font-mono whitespace-pre-wrap break-all">{log}</pre>
          {/each}
        </div>
      </div>
    </div>
  {/if}
</div>
