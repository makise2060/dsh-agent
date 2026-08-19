<script lang="ts">
  import { resolveClose } from '$lib/api/tauri';

  let remember = false;
  let busy = false;

  /** 关闭本覆盖层（由父组件通过可见性控制） */
  export let onCancel: (() => void) | undefined = undefined;

  async function choose(action: string) {
    if (busy) return;
    if (action === 'cancel') {
      onCancel?.();
      return;
    }
    busy = true;
    try {
      await resolveClose(action, remember);
    } finally {
      busy = false;
    }
  }
</script>

<!-- 关闭确认覆盖层：退出 / 最小化到托盘 / 取消 -->
<div
  class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm"
  role="dialog"
  aria-modal="true"
  aria-label="关闭确认"
>
  <div
    class="w-80 rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 p-6 shadow-2xl"
  >
    <h2 class="text-base font-semibold text-gray-900 dark:text-gray-100">关闭 DSH Agent？</h2>
    <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">
      最小化到托盘后 dsh 服务会在后台继续运行。
    </p>

    <div class="mt-5 flex flex-col gap-2">
      <button
        class="rounded-md bg-brand-600 px-4 py-2 text-sm font-medium text-white hover:bg-brand-700 disabled:opacity-50"
        disabled={busy}
        on:click={() => choose('quit')}
      >
        退出（停止 dsh）
      </button>
      <button
        class="rounded-md border border-gray-300 dark:border-gray-600 px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 disabled:opacity-50"
        disabled={busy}
        on:click={() => choose('tray')}
      >
        最小化到托盘
      </button>
      <button
        class="rounded-md px-4 py-2 text-sm font-medium text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700"
        on:click={() => choose('cancel')}
      >
        取消
      </button>
    </div>

    <label class="mt-4 flex cursor-pointer items-center gap-2 text-xs text-gray-500 dark:text-gray-400">
      <input type="checkbox" bind:checked={remember} class="h-3.5 w-3.5 rounded" />
      记住我的选择，下次不再询问
    </label>
  </div>
</div>
