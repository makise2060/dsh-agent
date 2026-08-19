<script lang="ts">
  import { onMount } from 'svelte';
  import { checkAppUpdate, getLogsDir, getNotifyOnDone, setNotifyOnDone } from '$lib/api/tauri';
  import { openPath, openUrl } from '@tauri-apps/plugin-opener';
  import type { UpdateInfo } from '$lib/api/types';

  const APP_VERSION = '1.2.1';
  const REPO_URL = 'https://github.com/makise2060/dsh-agent';
  const RELEASES_URL = `${REPO_URL}/releases`;
  const ISSUES_URL = `${REPO_URL}/issues`;

  let appUpdate: UpdateInfo | null = null;
  let checking = false;
  let checkError: string | null = null;
  let logsDir: string | null = null;

  // 任务完成通知开关
  let notifyOnDone = true;
  let notifyLoaded = false;

  onMount(async () => {
    try {
      notifyOnDone = await getNotifyOnDone();
    } catch (e) {
      console.error(e);
    } finally {
      notifyLoaded = true;
    }
  });

  async function handleToggleNotify() {
    const next = !notifyOnDone;
    notifyOnDone = next;
    try {
      await setNotifyOnDone(next);
    } catch (e) {
      console.error(e);
    }
  }

  async function handleCheckUpdate() {
    checking = true;
    checkError = null;
    try {
      appUpdate = await checkAppUpdate();
      if (appUpdate?.update_available) {
        await openUrl(`${RELEASES_URL}/latest`);
      }
    } catch (e) {
      checkError = String(e);
      console.error(e);
    } finally {
      checking = false;
    }
  }

  async function handleOpenLogs() {
    try {
      const dir = await getLogsDir();
      logsDir = dir;
      await openPath(dir);
    } catch (e) {
      console.error(e);
    }
  }
</script>

<div class="h-full overflow-y-auto bg-gray-50 dark:bg-gray-900 p-6 transition-colors">
  <div class="mx-auto max-w-2xl space-y-6">
    <!-- App Identity -->
    <div class="flex flex-col items-center gap-4 py-6">
      <!-- Logo -->
      <img
        src="/icon.png"
        alt="DSH Agent"
        class="h-24 w-24 rounded-2xl shadow-lg"
      />
      <div class="text-center">
        <h1 class="text-xl font-bold text-gray-800 dark:text-gray-200">DSH Agent</h1>
        <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">DeepSeek Harness 桌面客户端</p>
        <div class="mt-2 flex items-center justify-center gap-2">
          <span class="rounded-full bg-brand-100 dark:bg-brand-900/50 px-2.5 py-0.5 text-xs font-medium text-brand-700 dark:text-brand-300">v{APP_VERSION}</span>
          {#if appUpdate?.update_available}
            <button
              on:click={() => openUrl(`${RELEASES_URL}/latest`)}
              class="rounded-full bg-orange-100 dark:bg-orange-900/50 px-2.5 py-0.5 text-xs font-medium text-orange-700 dark:text-orange-300 hover:underline cursor-pointer"
            >
              → v{appUpdate.latest_version} 可用
            </button>
          {/if}
        </div>
      </div>
    </div>

    <!-- Action buttons -->
    <div class="flex flex-wrap items-center justify-center gap-3">
      <button
        on:click={() => openUrl(REPO_URL)}
        class="inline-flex items-center gap-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 px-4 py-2 text-xs font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors cursor-pointer"
      >
        <svg class="h-4 w-4" viewBox="0 0 24 24" fill="currentColor"><path d="M12 .297c-6.63 0-12 5.373-12 12 0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12"/></svg>
        GitHub
      </button>
      <button
        on:click={() => openUrl(RELEASES_URL)}
        class="inline-flex items-center gap-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 px-4 py-2 text-xs font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors cursor-pointer"
      >
        <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
        发布版本
      </button>
      <button
        on:click={() => openUrl(ISSUES_URL)}
        class="inline-flex items-center gap-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 px-4 py-2 text-xs font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors cursor-pointer"
      >
        <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
        问题反馈
      </button>
      <button
        on:click={handleCheckUpdate}
        disabled={checking}
        class="inline-flex items-center gap-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 px-4 py-2 text-xs font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors disabled:opacity-50 cursor-pointer"
      >
        <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"/><polyline points="1 20 1 14 7 14"/><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/></svg>
        {checking ? '检查中...' : '检查更新'}
      </button>
      <button
        on:click={handleOpenLogs}
        class="inline-flex items-center gap-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 px-4 py-2 text-xs font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors cursor-pointer"
      >
        <svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/></svg>
        查看日志
      </button>
    </div>

    <!-- Settings: task-completion notification -->
    <div class="rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 p-4 transition-colors">
      <div class="flex items-center justify-between gap-4">
        <div>
          <h2 class="text-sm font-bold text-gray-800 dark:text-gray-200">任务完成通知</h2>
          <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">
            任务完成时弹出系统通知并闪烁托盘图标（窗口在前台时不打扰）
          </p>
        </div>
        <button
          role="switch"
          aria-checked={notifyOnDone}
          aria-label="任务完成通知开关"
          disabled={!notifyLoaded}
          on:click={handleToggleNotify}
          class="relative inline-flex h-6 w-11 shrink-0 items-center rounded-full transition-colors disabled:opacity-50 cursor-pointer {notifyOnDone ? 'bg-brand-600' : 'bg-gray-300 dark:bg-gray-600'}"
        >
          <span
            class="inline-block h-4 w-4 transform rounded-full bg-white shadow transition-transform {notifyOnDone ? 'translate-x-6' : 'translate-x-1'}"
          ></span>
        </button>
      </div>
    </div>

    <!-- Logs dir hint -->
    {#if logsDir}
      <div class="rounded-lg border border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50 p-3 text-center">
        <p class="text-xs text-gray-500 dark:text-gray-400 font-mono break-all">{logsDir}</p>
      </div>
    {/if}

    <!-- Update status -->
    {#if checking}
      <div class="text-center text-xs text-gray-400 dark:text-gray-500">正在检查最新版本...</div>
    {:else if checkError}
      <div class="rounded-lg border border-red-200 dark:border-red-800 bg-red-50 dark:bg-red-900/30 p-4 text-center">
        <p class="text-sm font-medium text-red-700 dark:text-red-400">检查更新失败</p>
        <p class="mt-1 text-xs text-red-500 dark:text-red-400 font-mono break-all">{checkError}</p>
      </div>
    {:else if appUpdate?.update_available}
      <div class="rounded-lg border border-orange-200 dark:border-orange-800 bg-orange-50 dark:bg-orange-900/30 p-4 text-center">
        <p class="text-sm font-medium text-orange-700 dark:text-orange-400">发现新版本 v{appUpdate.latest_version}</p>
        <p class="mt-1 text-xs text-orange-600 dark:text-orange-400">已为你打开下载页面</p>
        {#if appUpdate.release_notes}
          <details class="mt-2 text-left">
            <summary class="cursor-pointer text-xs text-orange-600 dark:text-orange-400 select-none">查看更新内容</summary>
            <pre class="mt-2 max-h-60 overflow-y-auto whitespace-pre-wrap break-all rounded bg-white/60 dark:bg-black/30 p-2 text-xs text-gray-600 dark:text-gray-300">{appUpdate.release_notes}</pre>
          </details>
        {/if}
      </div>
    {:else if appUpdate}
      <div class="rounded-lg border border-green-200 dark:border-green-800 bg-green-50 dark:bg-green-900/30 p-4 text-center">
        <p class="text-sm font-medium text-green-700 dark:text-green-400">✓ 已是最新版本</p>
      </div>
    {/if}

    <!-- Tech stack -->
    <div class="rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 p-4 transition-colors">
      <h2 class="text-sm font-bold text-gray-800 dark:text-gray-200 mb-3">技术栈</h2>
      <div class="flex flex-wrap gap-2">
        <span class="rounded-md bg-gray-100 dark:bg-gray-700 px-2 py-1 text-xs text-gray-600 dark:text-gray-400">Tauri 2.x</span>
        <span class="rounded-md bg-gray-100 dark:bg-gray-700 px-2 py-1 text-xs text-gray-600 dark:text-gray-400">Svelte 5</span>
        <span class="rounded-md bg-gray-100 dark:bg-gray-700 px-2 py-1 text-xs text-gray-600 dark:text-gray-400">Rust</span>
        <span class="rounded-md bg-gray-100 dark:bg-gray-700 px-2 py-1 text-xs text-gray-600 dark:text-gray-400">Tailwind CSS</span>
        <span class="rounded-md bg-gray-100 dark:bg-gray-700 px-2 py-1 text-xs text-gray-600 dark:text-gray-400">Inno Setup</span>
      </div>
    </div>

    <!-- Footer -->
    <div class="pb-4 text-center">
      <p class="text-xs text-gray-400 dark:text-gray-500">
        Made by DrPepper · MIT License
      </p>
    </div>
  </div>
</div>
