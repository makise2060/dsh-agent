<script lang="ts">
  export let status: 'ok' | 'warn' | 'error';
  export let label: string;

  const colors = {
    ok: { dot: 'bg-green-500', text: 'text-green-600' },
    warn: { dot: 'bg-yellow-500', text: 'text-yellow-600' },
    error: { dot: 'bg-red-500', text: 'text-red-600' }
  };

  $: c = colors[status];
</script>

<span class="inline-flex items-center gap-1.5">
  <span class="inline-block h-2 w-2 rounded-full {c.dot}"></span>
  <span class="text-xs font-medium {c.text}">{label}</span>
</span>
