<script lang="ts">
  import StatusBadge from '$lib/components/StatusBadge.svelte';

  export let title: string;
  export let icon: string;
  export let badgeStatus: 'ok' | 'warn' | 'error';
  export let badgeLabel: string;
  export let highlight: 'none' | 'warn' | 'error' = 'none';
  export let path: string | null = null;

  const highlightClasses = {
    none: 'border-gray-200 dark:border-gray-700',
    warn: 'border-orange-300 dark:border-orange-700',
    error: 'border-red-300 dark:border-red-700'
  };
</script>

<div
  class="rounded-lg border bg-white dark:bg-gray-800 p-5 transition-colors {highlightClasses[highlight]}"
>
  <!-- 标题行 -->
  <div class="flex items-center justify-between gap-3">
    <div class="flex items-center gap-2">
      <svg class="h-4 w-4 text-gray-400 dark:text-gray-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        {@html icon}
      </svg>
      <span class="text-sm font-medium text-gray-700 dark:text-gray-300">{title}</span>
    </div>
    <StatusBadge status={badgeStatus} label={badgeLabel} variant="pill" />
  </div>

  <!-- 路径行 -->
  {#if path}
    <p class="mt-2 truncate text-xs text-gray-400 dark:text-gray-500 font-mono">{path}</p>
  {/if}

  <!-- 正文 -->
  <div class="mt-3 space-y-3">
    <slot />
  </div>

  <!-- 操作区 -->
  {#if $$slots.actions}
    <div class="mt-4 flex flex-wrap items-center gap-2 border-t border-gray-100 dark:border-gray-700 pt-3">
      <slot name="actions" />
    </div>
  {/if}
</div>
