<script lang="ts">
  export let status: 'ok' | 'warn' | 'error';
  export let label: string;
  export let variant: 'dot' | 'pill' = 'dot';

  const colors = {
    ok: { dot: 'bg-green-500', text: 'text-green-600 dark:text-green-400', pill: 'bg-green-50 dark:bg-green-900/30 text-green-700 dark:text-green-300 border-green-200 dark:border-green-800' },
    warn: { dot: 'bg-yellow-500', text: 'text-yellow-600 dark:text-yellow-400', pill: 'bg-yellow-50 dark:bg-yellow-900/30 text-yellow-700 dark:text-yellow-300 border-yellow-200 dark:border-yellow-800' },
    error: { dot: 'bg-red-500', text: 'text-red-600 dark:text-red-400', pill: 'bg-red-50 dark:bg-red-900/30 text-red-700 dark:text-red-300 border-red-200 dark:border-red-800' }
  };

  $: c = colors[status];
</script>

{#if variant === 'pill'}
  <span class="inline-flex items-center rounded-full border px-2 py-0.5 text-xs font-medium {c.pill}">
    {label}
  </span>
{:else}
  <span class="inline-flex items-center gap-1.5">
    <span class="inline-block h-2 w-2 rounded-full {c.dot}"></span>
    <span class="text-xs font-medium {c.text}">{label}</span>
  </span>
{/if}
