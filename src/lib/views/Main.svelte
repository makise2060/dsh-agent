<script lang="ts">
  import { onMount } from 'svelte';
  import { processState } from '$lib/stores/app';
  import { startDsh, onProcessStateChanged } from '$lib/api/tauri';
  import LoadingScreen from '$lib/components/LoadingScreen.svelte';

  let loading = true;
  let error: string | null = null;

  onMount(async () => {
    try {
      const state = await startDsh();
      processState.set(state);
      if (state.status === 'Running' && state.url) {
        loading = false;
      } else if (state.status === 'Failed') {
        error = state.error ?? '启动失败';
        loading = false;
      }
    } catch (e) {
      error = String(e);
      loading = false;
    }

    const unlisten = await onProcessStateChanged((state) => {
      processState.set(state);
      if (state.status === 'Running' && state.url) {
        loading = false;
        error = null;
      } else if (state.status === 'Failed') {
        error = state.error ?? '启动失败';
        loading = false;
      }
    });
    return () => unlisten();
  });
</script>

{#if loading}
  <LoadingScreen />
{:else if error}
  <div class="flex h-full flex-col items-center justify-center gap-4 bg-gray-50 p-8">
    <div class="rounded-lg border border-red-200 bg-red-50 p-6 text-center">
      <p class="text-sm font-medium text-red-700">dsh 启动失败</p>
      <p class="mt-2 max-w-md text-xs text-red-500">{error}</p>
    </div>
    <button
      class="rounded-md bg-brand-600 px-4 py-2 text-xs font-medium text-white hover:bg-brand-700"
      on:click={() => {
        loading = true;
        error = null;
        startDsh().then((s) => {
          processState.set(s);
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
