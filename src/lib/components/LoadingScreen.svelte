<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { onDshStdout, onProcessStateChanged } from '$lib/api/tauri';

  let logs: string[] = [];
  let port: number | null = null;
  let unlistenStdout: (() => void) | null = null;
  let unlistenState: (() => void) | null = null;

  onMount(async () => {
    unlistenStdout = await onDshStdout((line: string) => {
      logs = [...logs, line];
      // Auto-scroll to bottom
      setTimeout(() => {
        const el = document.getElementById('dsh-log-scroll');
        if (el) el.scrollTop = el.scrollHeight;
      }, 0);
    });

    unlistenState = await onProcessStateChanged((state) => {
      if (state.port) port = state.port;
    });
  });

  onDestroy(() => {
    unlistenStdout?.();
    unlistenState?.();
  });

  // Startup steps for display
  const steps = [
    { label: '初始化运行环境', detail: '检测 Node.js / npx 可用性' },
    { label: '启动 dsh 服务', detail: 'npx @deepseek-ai/dsh web --port 0' },
    { label: '等待服务就绪', detail: '监听 stdout 获取随机端口' },
    { label: '加载 Web UI', detail: '连接 iframe 到本地服务' }
  ];

  // Determine current step based on logs
  $: currentStep = (() => {
    if (port) return 4;
    if (logs.length > 0) return 3;
    return 2;
  })();
</script>

<div class="flex h-full flex-col items-center justify-center gap-4 bg-gray-50 dark:bg-gray-900 p-8">
  <!-- Spinner + Title -->
  <div class="flex items-center gap-3">
    <div class="h-6 w-6 animate-spin rounded-full border-2 border-brand-500 border-t-transparent"></div>
    <span class="text-sm font-medium text-gray-700 dark:text-gray-300">正在启动 DeepSeek Harness...</span>
  </div>

  <!-- Steps -->
  <div class="w-full max-w-md space-y-2">
    {#each steps as step, i}
      <div class="flex items-start gap-3">
        <!-- Step indicator -->
        <div class="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full text-xs font-medium
          {i < currentStep - 1 ? 'bg-green-500 text-white' : ''}
          {i === currentStep - 1 ? 'bg-brand-500 text-white animate-pulse' : ''}
          {i >= currentStep ? 'bg-gray-200 dark:bg-gray-700 text-gray-400 dark:text-gray-500' : ''}">
          {#if i < currentStep - 1}
            ✓
          {:else if i === currentStep - 1}
            {i + 1}
          {:else}
            {i + 1}
          {/if}
        </div>
        <div class="flex-1">
          <p class="text-xs font-medium
            {i < currentStep - 1 ? 'text-green-600 dark:text-green-400' : ''}
            {i === currentStep - 1 ? 'text-brand-600 dark:text-brand-400' : ''}
            {i >= currentStep ? 'text-gray-400 dark:text-gray-500' : ''}">
            {step.label}
          </p>
          <p class="text-xs text-gray-400 dark:text-gray-500 mt-0.5 font-mono">{step.detail}</p>
        </div>
      </div>
    {/each}
  </div>

  <!-- Port info -->
  {#if port}
    <div class="rounded-md bg-green-50 dark:bg-green-900/30 border border-green-200 dark:border-green-800 px-3 py-1.5">
      <span class="text-xs text-green-700 dark:text-green-400 font-medium">✓ 服务已就绪 · 端口 :{port}</span>
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
          <span class="text-xs text-gray-400 ml-1">dsh stdout</span>
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
