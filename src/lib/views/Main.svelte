<script lang="ts">
  import { onMount } from 'svelte';
  import { processState } from '$lib/stores/app';
  import { bundleStatus } from '$lib/stores/plugins';
  import {
    startBootstrap,
    getDshStatus,
    onProcessStateChanged,
    onBootstrapFailed,
    checkBundleStatus
  } from '$lib/api/tauri';
  import LoadingScreen from '$lib/components/LoadingScreen.svelte';

  let loading = true;
  let error: string | null = null;

  onMount(() => {
    let unlistenState: (() => void) | undefined;
    let unlistenFailed: (() => void) | undefined;

    // 启动时静默检测界面插件全家桶状态（纯文件读取，不自动安装），
    // 插件市场页会基于此展示状态徽标
    checkBundleStatus()
      .then((s) => bundleStatus.set(s))
      .catch(() => {});

    (async () => {
      try {
        // 先查后端状态：已在运行则直接进入（如托盘驻留后重新打开窗口）
        const state = await getDshStatus();
        if (state.status === 'Running' && state.url) {
          processState.set(state);
          loading = false;
        } else {
          // 未运行：走引导流水线（检查/下载 Node → 装 dsh → 装插件 → 启动 dsh）
          await startBootstrap();
          // 引导成功后会 emit process-state-changed(Running)，由下方监听接管；
          // 若事件在监听注册前到达，这里再兜底查一次
          const after = await getDshStatus();
          if (after.status === 'Running' && after.url) {
            processState.set(after);
            loading = false;
          }
        }
      } catch (e) {
        error = String(e);
        loading = false;
      }

      unlistenState = await onProcessStateChanged((state) => {
        processState.set(state);
        if (state.status === 'Running' && state.url) {
          loading = false;
          error = null;
        } else if (state.status === 'Failed') {
          error = state.error ?? '启动失败';
          loading = false;
        }
      });

      // 引导失败事件（bootstrap:failed）
      unlistenFailed = await onBootstrapFailed((message) => {
        error = message;
        loading = false;
      });
    })();

    return () => {
      unlistenState?.();
      unlistenFailed?.();
    };
  });
</script>

{#if loading}
  <LoadingScreen />
{:else if error}
  <div class="flex h-full flex-col items-center justify-center gap-4 bg-gray-50 dark:bg-gray-900 p-8">
    <div class="rounded-lg border border-red-200 dark:border-red-800 bg-red-50 dark:bg-red-900/30 p-6 text-center">
      <p class="text-sm font-medium text-red-700 dark:text-red-400">dsh 启动失败</p>
      <p class="mt-2 max-w-md text-xs text-red-500 dark:text-red-400 whitespace-pre-wrap break-all">{error}</p>
    </div>
    <button
      class="rounded-md bg-brand-600 px-4 py-2 text-xs font-medium text-white hover:bg-brand-700"
      on:click={() => {
        loading = true;
        error = null;
        startBootstrap()
          .then(() => getDshStatus())
          .then((s) => {
            processState.set(s);
            if (s.status === 'Running' && s.url) loading = false;
          })
          .catch((e) => {
            error = String(e);
            loading = false;
          });
      }}
    >
      重试
    </button>
  </div>
{:else if $processState.url}
  <iframe
    src={$processState.url}
    title="DeepSeek Harness Web UI"
    class="h-full w-full border-0"
    allow="clipboard-read; clipboard-write"
  ></iframe>
{/if}
