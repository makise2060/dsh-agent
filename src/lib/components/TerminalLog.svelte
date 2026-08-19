<script lang="ts">
  import { tick } from 'svelte';

  export let lines: string[] = [];
  export let title = '日志';
  export let collapsible = false;
  export let collapsed = true;
  export let maxHeight = 'max-h-48';
  export let autoScroll = true;

  let logEl: HTMLDivElement | undefined;

  // 行数变化后滚到底部
  $: if (autoScroll && lines.length > 0) {
    tick().then(() => {
      if (logEl) logEl.scrollTop = logEl.scrollHeight;
    });
  }
</script>

{#if collapsible}
  <div class="overflow-hidden rounded-lg border border-gray-700 dark:border-gray-800 bg-gray-900 dark:bg-black">
    <button
      class="flex w-full cursor-pointer items-center gap-2 border-b border-gray-700 dark:border-gray-800 px-3 py-1.5"
      on:click={() => (collapsed = !collapsed)}
    >
      <span class="flex gap-1.5">
        <span class="h-2.5 w-2.5 rounded-full bg-red-400"></span>
        <span class="h-2.5 w-2.5 rounded-full bg-yellow-400"></span>
        <span class="h-2.5 w-2.5 rounded-full bg-green-400"></span>
      </span>
      <span class="text-xs text-gray-400 ml-1">{title}</span>
      <svg
        class="ml-auto h-3.5 w-3.5 text-gray-500 transition-transform {collapsed ? '' : 'rotate-180'}"
        viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
      >
        <polyline points="6 9 12 15 18 9" />
      </svg>
    </button>
    {#if !collapsed}
      <div bind:this={logEl} class="overflow-y-auto p-3 space-y-0.5 {maxHeight}">
        {#each lines as line}
          <pre class="text-xs text-green-400 font-mono whitespace-pre-wrap break-all">{line}</pre>
        {/each}
      </div>
    {/if}
  </div>
{:else}
  <div class="overflow-hidden rounded-lg border border-gray-700 dark:border-gray-800 bg-gray-900 dark:bg-black">
    <div class="flex items-center gap-2 border-b border-gray-700 dark:border-gray-800 px-3 py-1.5">
      <span class="flex gap-1.5">
        <span class="h-2.5 w-2.5 rounded-full bg-red-400"></span>
        <span class="h-2.5 w-2.5 rounded-full bg-yellow-400"></span>
        <span class="h-2.5 w-2.5 rounded-full bg-green-400"></span>
      </span>
      <span class="text-xs text-gray-400 ml-1">{title}</span>
    </div>
    <div bind:this={logEl} class="overflow-y-auto p-3 space-y-0.5 {maxHeight}">
      {#each lines as line}
        <pre class="text-xs text-green-400 font-mono whitespace-pre-wrap break-all">{line}</pre>
      {/each}
    </div>
  </div>
{/if}
