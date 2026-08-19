<script lang="ts">
  export let variant: 'primary' | 'secondary' | 'ghost' = 'secondary';
  export let size: 'xs' | 'sm' = 'sm';
  export let disabled = false;
  export let loading = false;
  /** lucide outline path（stroke 风格），非 null 时在文字前渲染小图标 */
  export let icon: string | null = null;

  const variants = {
    primary:
      'bg-brand-600 text-white hover:bg-brand-700 disabled:opacity-50',
    secondary:
      'border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 disabled:opacity-50',
    ghost:
      'text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 disabled:opacity-50'
  };

  const sizes = {
    xs: 'px-2 py-1 text-xs',
    sm: 'px-3 py-1.5 text-xs'
  };

  $: classes = `${variants[variant]} ${sizes[size]}`;
  $: finalDisabled = disabled || loading;
</script>

<button
  class="inline-flex cursor-pointer items-center gap-1.5 rounded-md font-medium transition-colors {classes}"
  disabled={finalDisabled}
  on:click
>
  {#if loading}
    <span class="h-3 w-3 animate-spin rounded-full border-2 border-current border-t-transparent"></span>
  {:else if icon}
    <svg class="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      {@html icon}
    </svg>
  {/if}
  <slot />
</button>
